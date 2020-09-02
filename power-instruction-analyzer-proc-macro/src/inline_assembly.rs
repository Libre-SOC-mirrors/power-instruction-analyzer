// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Write,
    hash::{Hash, Hasher},
    marker::PhantomPinned,
    ops::{Add, AddAssign, Deref, DerefMut},
    pin::Pin,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};
use syn::LitStr;

pub(crate) trait ToAssembly {
    fn to_assembly(&self) -> Assembly;
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct AssemblyArgId(u64);

impl AssemblyArgId {
    pub(crate) fn new() -> Self {
        // don't start at zero to help avoid confusing id with indexes
        static NEXT_ID: AtomicU64 = AtomicU64::new(1000);
        AssemblyArgId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

macro_rules! impl_assembly_arg {
    (
        $vis:vis struct $name:ident {
            tokens: TokenStream,
            $(
                $id:ident: AssemblyArgId,
            )?
        }
    ) => {
        #[derive(Debug, Clone)]
        $vis struct $name {
            tokens: TokenStream,
            $($id: AssemblyArgId,)?
        }

        impl $name {
            $vis fn new(tokens: impl ToTokens) -> Self {
                tokens.into_token_stream().into()
            }
        }

        impl ToTokens for $name {
            fn to_token_stream(&self) -> TokenStream {
                self.tokens.clone()
            }

            fn into_token_stream(self) -> TokenStream {
                self.tokens
            }

            fn to_tokens(&self, tokens: &mut TokenStream) {
                self.tokens.to_tokens(tokens)
            }
        }

        impl From<TokenStream> for $name {
            fn from(tokens: TokenStream) -> Self {
                Self {
                    tokens,
                    $($id: AssemblyArgId::new(),)?
                }
            }
        }
    };
}

impl_assembly_arg! {
    pub(crate) struct AssemblyInputArg {
        tokens: TokenStream,
        id: AssemblyArgId,
    }
}

impl_assembly_arg! {
    pub(crate) struct AssemblyOutputArg {
        tokens: TokenStream,
        id: AssemblyArgId,
    }
}

impl_assembly_arg! {
    pub(crate) struct AssemblyClobber {
        tokens: TokenStream,
    }
}

#[derive(Debug, Clone)]
pub(crate) enum AssemblyTextFragment {
    Text(String),
    ArgIndex(AssemblyArgId),
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Assembly {
    text_fragments: Vec<AssemblyTextFragment>,
    inputs: Vec<AssemblyInputArg>,
    outputs: Vec<AssemblyOutputArg>,
    clobbers: Vec<AssemblyClobber>,
}

impl From<String> for Assembly {
    fn from(text: String) -> Self {
        Self {
            text_fragments: vec![AssemblyTextFragment::Text(text)],
            ..Self::default()
        }
    }
}

impl From<&'_ str> for Assembly {
    fn from(text: &str) -> Self {
        String::from(text).into()
    }
}

impl Assembly {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn to_text(&self) -> String {
        let mut id_index_map = HashMap::new();
        for (index, id) in self
            .outputs
            .iter()
            .map(|v| v.id)
            .chain(self.inputs.iter().map(|v| v.id))
            .enumerate()
        {
            if let Some(old_index) = id_index_map.insert(id, index) {
                panic!(
                    "duplicate id in inline assembly arguments: #{} and #{}\n{:#?}",
                    old_index, index, self
                );
            }
        }
        let mut retval = String::new();
        for text_fragment in &self.text_fragments {
            match text_fragment {
                AssemblyTextFragment::Text(text) => retval += text,
                AssemblyTextFragment::ArgIndex(id) => {
                    if let Some(index) = id_index_map.get(id) {
                        write!(retval, "{}", index).unwrap();
                    } else {
                        panic!(
                            "unknown id in inline assembly arguments: id={:?}\n{:#?}",
                            id, self
                        );
                    }
                }
            }
        }
        retval
    }
}

impl AddAssign<&'_ Assembly> for Assembly {
    fn add_assign(&mut self, rhs: &Assembly) {
        let Self {
            text_fragments,
            inputs,
            outputs,
            clobbers,
        } = self;
        text_fragments.reserve(rhs.text_fragments.len());
        for text_fragment in &rhs.text_fragments {
            match *text_fragment {
                AssemblyTextFragment::Text(ref rhs_text) => {
                    if let Some(AssemblyTextFragment::Text(text)) = text_fragments.last_mut() {
                        *text += rhs_text;
                    } else {
                        text_fragments.push(AssemblyTextFragment::Text(rhs_text.clone()));
                    }
                }
                AssemblyTextFragment::ArgIndex(id) => {
                    self.text_fragments.push(AssemblyTextFragment::ArgIndex(id));
                }
            }
        }
        inputs.extend_from_slice(&rhs.inputs);
        outputs.extend_from_slice(&rhs.outputs);
        clobbers.extend_from_slice(&rhs.clobbers);
    }
}

impl AddAssign<Assembly> for Assembly {
    fn add_assign(&mut self, rhs: Assembly) {
        *self += &rhs;
    }
}

impl Add for Assembly {
    type Output = Assembly;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl Add<&'_ Assembly> for Assembly {
    type Output = Assembly;

    fn add(mut self, rhs: &Assembly) -> Self::Output {
        self += rhs;
        self
    }
}

impl Add<Assembly> for &'_ Assembly {
    type Output = Assembly;

    fn add(self, rhs: Assembly) -> Self::Output {
        Assembly::clone(self) + rhs
    }
}

impl Add<&'_ Assembly> for &'_ Assembly {
    type Output = Assembly;

    fn add(self, rhs: &Assembly) -> Self::Output {
        Assembly::clone(self) + rhs
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AssemblyWithTextSpan {
    pub(crate) asm: Assembly,
    pub(crate) text_span: Span,
}

impl Deref for AssemblyWithTextSpan {
    type Target = Assembly;

    fn deref(&self) -> &Self::Target {
        &self.asm
    }
}

impl DerefMut for AssemblyWithTextSpan {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.asm
    }
}

impl ToTokens for AssemblyWithTextSpan {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            asm:
                Assembly {
                    text_fragments: _,
                    inputs,
                    outputs,
                    clobbers,
                },
            text_span,
        } = self;
        let text = LitStr::new(&self.to_text(), text_span.clone());
        let value = quote! {
            llvm_asm!(#text : #(#outputs),* : #(#inputs),* : #(#clobbers),*)
        };
        value.to_tokens(tokens);
    }
}
