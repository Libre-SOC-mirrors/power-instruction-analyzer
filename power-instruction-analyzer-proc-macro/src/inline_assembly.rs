// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::{
    collections::HashMap,
    fmt::Write,
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU64, Ordering},
};
use syn::{punctuated::Punctuated, LitStr, Token};

macro_rules! append_assembly {
    ($retval:ident;) => {};
    ($retval:ident; $lit:literal $($tt:tt)*) => {
        $crate::inline_assembly::ToAssembly::append_to($lit, &mut $retval);
        append_assembly!($retval; $($tt)*);
    };
    ($retval:ident; input($arg_id:ident = {$($arg_tt:tt)*}) $($tt:tt)*) => {
        {
            let (arg, arg_id) = $crate::inline_assembly::Assembly::make_input(quote! {$($arg_tt)*});
            $crate::inline_assembly::ToAssembly::append_to(&arg, &mut $retval);
            $arg_id = arg_id;
        }
        append_assembly!($retval; $($tt)*);
    };
    ($retval:ident; input{$($arg_tt:tt)*} $($tt:tt)*) => {
        {
            let (arg, _arg_id) = $crate::inline_assembly::Assembly::make_input(quote! {$($arg_tt)*});
            $crate::inline_assembly::ToAssembly::append_to(&arg, &mut $retval);
        }
        append_assembly!($retval; $($tt)*);
    };
    ($retval:ident; output($arg_id:ident = {$($arg_tt:tt)*}) $($tt:tt)*) => {
        {
            let (arg, arg_id) = $crate::inline_assembly::Assembly::make_output(quote! {$($arg_tt)*});
            $crate::inline_assembly::ToAssembly::append_to(&arg, &mut $retval);
            $arg_id = arg_id;
        }
        append_assembly!($retval; $($tt)*);
    };
    ($retval:ident; output{$($arg_tt:tt)*} $($tt:tt)*) => {
        {
            let (arg, _arg_id) = $crate::inline_assembly::Assembly::make_output(quote! {$($arg_tt)*});
            $crate::inline_assembly::ToAssembly::append_to(&arg, &mut $retval);
        }
        append_assembly!($retval; $($tt)*);
    };
    ($retval:ident; clobber{$($arg_tt:tt)*} $($tt:tt)*) => {
        $crate::inline_assembly::ToAssembly::append_to(
            &$crate::inline_assembly::Assembly::make_clobber(quote::quote! {$($arg_tt)*}),
            &mut $retval
        );
        append_assembly!($retval; $($tt)*);
    };
    ($retval:ident; ($arg_id:ident) $($tt:tt)*) => {
        $crate::inline_assembly::ToAssembly::append_to(&$arg_id, &mut $retval);
        append_assembly!($retval; $($tt)*);
    };
}

macro_rules! assembly {
    () => {
        $crate::inline_assembly::Assembly::new()
    };
    ($($tt:tt)*) => {
        {
            let mut retval = $crate::inline_assembly::Assembly::new();
            append_assembly!(retval; $($tt)*);
            retval
        }
    };
}

pub(crate) trait ToAssembly {
    /// appends `self` to `retval`
    fn append_to(&self, retval: &mut Assembly);

    fn to_assembly(&self) -> Assembly {
        let mut retval = Assembly::default();
        self.append_to(&mut retval);
        retval
    }

    fn into_assembly(self) -> Assembly
    where
        Self: Sized,
    {
        let mut retval = Assembly::default();
        self.append_to(&mut retval);
        retval
    }
}

impl<T: ToAssembly + ?Sized> ToAssembly for &'_ T {
    fn append_to(&self, retval: &mut Assembly) {
        (**self).append_to(retval);
    }

    fn to_assembly(&self) -> Assembly {
        (**self).to_assembly()
    }
}

impl<T: ToAssembly + ?Sized> ToAssembly for &'_ mut T {
    fn append_to(&self, retval: &mut Assembly) {
        (**self).append_to(retval);
    }

    fn to_assembly(&self) -> Assembly {
        (**self).to_assembly()
    }
}

impl<T: ToAssembly> ToAssembly for Box<T> {
    fn append_to(&self, retval: &mut Assembly) {
        (**self).append_to(retval);
    }

    fn to_assembly(&self) -> Assembly {
        (**self).to_assembly()
    }

    fn into_assembly(self) -> Assembly {
        (*self).into_assembly()
    }
}

impl ToAssembly for str {
    fn append_to(&self, retval: &mut Assembly) {
        if let Some(AssemblyTextFragment::Text(text)) = retval.text_fragments.last_mut() {
            *text += self;
        } else {
            retval
                .text_fragments
                .push(AssemblyTextFragment::Text(self.into()));
        }
    }
}

impl ToAssembly for String {
    fn append_to(&self, retval: &mut Assembly) {
        str::append_to(&self, retval)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct AssemblyMetavariableId(u64);

impl AssemblyMetavariableId {
    pub(crate) fn new() -> Self {
        // don't start at zero to help avoid confusing id with indexes
        static NEXT_ID: AtomicU64 = AtomicU64::new(10000);
        AssemblyMetavariableId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl ToAssembly for AssemblyMetavariableId {
    fn append_to(&self, retval: &mut Assembly) {
        retval
            .text_fragments
            .push(AssemblyTextFragment::Metavariable(*self));
    }
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

impl ToAssembly for AssemblyArgId {
    fn append_to(&self, retval: &mut Assembly) {
        retval
            .text_fragments
            .push(AssemblyTextFragment::ArgIndex(*self));
    }
}

macro_rules! impl_assembly_arg {
    (
        struct $name:ident {
            tokens: TokenStream,
            $(
                $id:ident: AssemblyArgId,
            )?
        }
    ) => {
        #[derive(Debug, Clone)]
        struct $name {
            tokens: TokenStream,
            $($id: AssemblyArgId,)?
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
    struct AssemblyInputArg {
        tokens: TokenStream,
        id: AssemblyArgId,
    }
}

impl_assembly_arg! {
    struct AssemblyOutputArg {
        tokens: TokenStream,
        id: AssemblyArgId,
    }
}

impl_assembly_arg! {
    struct AssemblyClobber {
        tokens: TokenStream,
    }
}

#[derive(Debug, Clone)]
pub(crate) enum AssemblyTextFragment {
    Text(String),
    ArgIndex(AssemblyArgId),
    Metavariable(AssemblyMetavariableId),
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

impl From<AssemblyArgId> for Assembly {
    fn from(arg_id: AssemblyArgId) -> Self {
        Self {
            text_fragments: vec![AssemblyTextFragment::ArgIndex(arg_id)],
            ..Self::default()
        }
    }
}

impl From<&'_ AssemblyArgId> for Assembly {
    fn from(arg_id: &AssemblyArgId) -> Self {
        Self::from(*arg_id)
    }
}

impl From<AssemblyMetavariableId> for Assembly {
    fn from(arg_id: AssemblyMetavariableId) -> Self {
        Self {
            text_fragments: vec![AssemblyTextFragment::Metavariable(arg_id)],
            ..Self::default()
        }
    }
}

impl From<&'_ AssemblyMetavariableId> for Assembly {
    fn from(arg_id: &AssemblyMetavariableId) -> Self {
        Self::from(*arg_id)
    }
}

impl Assembly {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn make_input(tokens: impl ToTokens) -> (Self, AssemblyArgId) {
        let input: AssemblyInputArg = tokens.into_token_stream().into();
        let id = input.id;
        (
            Self {
                text_fragments: vec![AssemblyTextFragment::ArgIndex(id)],
                inputs: vec![input],
                ..Self::default()
            },
            id,
        )
    }
    pub(crate) fn make_output(tokens: impl ToTokens) -> (Self, AssemblyArgId) {
        let output: AssemblyOutputArg = tokens.into_token_stream().into();
        let id = output.id;
        (
            Self {
                text_fragments: vec![AssemblyTextFragment::ArgIndex(id)],
                outputs: vec![output],
                ..Self::default()
            },
            id,
        )
    }
    pub(crate) fn make_clobber(tokens: impl ToTokens) -> Self {
        Self {
            clobbers: vec![tokens.into_token_stream().into()],
            ..Self::default()
        }
    }
    pub(crate) fn replace_metavariables<R>(
        &self,
        mut f: impl FnMut(AssemblyMetavariableId) -> Result<Assembly, R>,
    ) -> Result<Assembly, R> {
        let mut retval = self.args_without_text();
        for text_fragment in &self.text_fragments {
            match text_fragment {
                AssemblyTextFragment::Text(text) => text.append_to(&mut retval),
                AssemblyTextFragment::ArgIndex(id) => id.append_to(&mut retval),
                AssemblyTextFragment::Metavariable(id) => f(*id)?.append_to(&mut retval),
            }
        }
        Ok(retval)
    }
    pub(crate) fn args_without_text(&self) -> Assembly {
        Assembly {
            text_fragments: Vec::new(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            clobbers: self.clobbers.clone(),
        }
    }
    pub(crate) fn text_without_args(&self) -> Assembly {
        Assembly {
            text_fragments: self.text_fragments.clone(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            clobbers: Vec::new(),
        }
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
                AssemblyTextFragment::Metavariable(id) => {
                    panic!(
                        "metavariables are not allowed when converting \
                            assembly to text: metavariable id={:?}\n{:#?}",
                        id, self
                    );
                }
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

impl ToAssembly for Assembly {
    fn append_to(&self, retval: &mut Assembly) {
        retval.text_fragments.reserve(self.text_fragments.len());
        for text_fragment in &self.text_fragments {
            match *text_fragment {
                AssemblyTextFragment::Text(ref text) => text.append_to(retval),
                AssemblyTextFragment::Metavariable(id) => id.append_to(retval),
                AssemblyTextFragment::ArgIndex(id) => id.append_to(retval),
            }
        }
        retval.inputs.extend_from_slice(&self.inputs);
        retval.outputs.extend_from_slice(&self.outputs);
        retval.clobbers.extend_from_slice(&self.clobbers);
    }

    fn to_assembly(&self) -> Assembly {
        self.clone()
    }

    fn into_assembly(self) -> Assembly {
        self
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
        let mut args: Punctuated<TokenStream, Token![,]> = self
            .to_text()
            .lines()
            .map(|line| {
                quote_spanned! {*text_span=>
                    #line
                }
            })
            .collect();
        args.extend(outputs.iter().map(ToTokens::to_token_stream));
        args.extend(inputs.iter().map(ToTokens::to_token_stream));
        args.extend(clobbers.iter().map(ToTokens::to_token_stream));
        let value = quote! {
            asm!(#args)
        };
        value.to_tokens(tokens);
    }
}
