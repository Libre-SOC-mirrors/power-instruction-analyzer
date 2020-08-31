// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use std::{borrow::Cow, fmt, fmt::Write};
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Attribute, Error, ItemFn, LitStr, Token,
};

macro_rules! valid_enumerants_as_string {
    ($enumerant:ident) => {
        concat!("`", stringify!($enumerant), "`")
    };
    ($enumerant1:ident, $enumerant2:ident) => {
        concat!("`", stringify!($enumerant1), "` and `", stringify!($enumerant2), "`")
    };
    ($($enumerant:ident),+) => {
        valid_enumerants_as_string!((), ($($enumerant),+))
    };
    (($first_enumerant:ident, $($enumerant:ident,)+), ($last_enumerant:ident)) => {
        concat!(
            "`",
            stringify!($first_enumerant),
            $(
                "`, `",
                stringify!($enumerant),
            )+
            "`, and `",
            stringify!($last_enumerant),
            "`"
        )
    };
    (($($enumerants:ident,)*), ($next_enumerant:ident, $($rest:ident),*)) => {
        valid_enumerants_as_string!(($($enumerants,)* $next_enumerant,), ($($rest),*))
    };
    () => {
        "<nothing>"
    };
}

macro_rules! ident_enum {
    (
        #[parse_error_msg = $parse_error_msg:literal]
        enum $enum_name:ident {
            $(
                $enumerant:ident,
            )*
        }
    ) => {
        #[derive(Copy, Clone, Eq, PartialEq, Hash)]
        enum $enum_name<T = Span> {
            $(
                $enumerant(T),
            )*
        }

        impl<T> fmt::Debug for $enum_name<T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.name())
            }
        }

        impl<T> $enum_name<T> {
            fn enumerant(&self) -> $enum_name<()> {
                match self {
                    $(
                        $enum_name::$enumerant(_) => $enum_name::$enumerant(()),
                    )*
                }
            }
            fn name(&self) -> &'static str {
                match self {
                    $(
                        $enum_name::$enumerant(_) => stringify!($enumerant),
                    )*
                }
            }
        }

        impl $enum_name {
            fn into_ident(self) -> Ident {
                match self {
                    $(
                        $enum_name::$enumerant(span) => Ident::new(stringify!($enumerant), span),
                    )*
                }
            }
        }

        impl ToTokens for $enum_name<Span> {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                tokens.append(self.clone().into_ident());
            }
        }

        impl Parse for $enum_name<Span> {
            fn parse(input: ParseStream) -> syn::Result<Self> {
                let id: Ident = input.parse()?;
                $(
                    if id == stringify!($enumerant) {
                        return Ok($enum_name::$enumerant(id.span()));
                    }
                )*
                Err(Error::new_spanned(
                    id,
                    concat!(
                        $parse_error_msg,
                        ": valid values are: ",
                        valid_enumerants_as_string!($($enumerant),*)
                    )
                ))
            }
        }
    };
}

ident_enum! {
    #[parse_error_msg = "unknown instruction input"]
    enum InstructionInput {
        Ra,
        Rb,
        Rc,
        Carry,
    }
}

ident_enum! {
    #[parse_error_msg = "unknown instruction output"]
    enum InstructionOutput {
        Rt,
        Carry,
        Overflow,
        CR0,
    }
}

#[derive(Debug, Clone)]
enum AssemblyTextFragment {
    Text(String),
    InputIndex(usize),
    OutputIndex(usize),
}

struct InlineAssembly {
    text: Vec<AssemblyTextFragment>,
    text_span: Span,
    inputs: Vec<TokenStream>,
    outputs: Vec<TokenStream>,
    clobbers: Vec<TokenStream>,
}

impl fmt::Write for InlineAssembly {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Some(AssemblyTextFragment::Text(v)) = self.text.last_mut() {
            *v += s;
        } else {
            self.text.push(AssemblyTextFragment::Text(String::from(s)));
        }
        Ok(())
    }
}

impl InlineAssembly {
    fn new(text_span: Span) -> Self {
        Self {
            text: Vec::new(),
            text_span,
            inputs: Vec::new(),
            outputs: Vec::new(),
            clobbers: Vec::new(),
        }
    }
    fn to_text(&self) -> String {
        let mut retval = String::new();
        for text in &self.text {
            match text {
                AssemblyTextFragment::Text(text) => retval += text,
                AssemblyTextFragment::InputIndex(index) => {
                    write!(retval, "{}", index + self.outputs.len()).unwrap();
                }
                AssemblyTextFragment::OutputIndex(index) => write!(retval, "{}", index).unwrap(),
            }
        }
        retval
    }
    fn write_input_index(&mut self, index: usize) -> fmt::Result {
        self.text.push(AssemblyTextFragment::InputIndex(index));
        Ok(())
    }
    fn write_output_index(&mut self, index: usize) -> fmt::Result {
        self.text.push(AssemblyTextFragment::OutputIndex(index));
        Ok(())
    }
}

impl ToTokens for InlineAssembly {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            text: _,
            text_span,
            inputs,
            outputs,
            clobbers,
        } = self;
        let text = LitStr::new(&self.to_text(), text_span.clone());
        let value = quote! {
            llvm_asm!(#text : #(#outputs),* : #(#inputs),* : #(#clobbers),*)
        };
        value.to_tokens(tokens);
    }
}

#[derive(Debug)]
struct Instruction {
    enumerant: Ident,
    fn_name: Ident,
    inputs: Punctuated<InstructionInput, Token!(,)>,
    outputs: Punctuated<InstructionOutput, Token!(,)>,
    instruction_name: LitStr,
}

impl Parse for Instruction {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token!(#)>()?;
        let enumerant_attr_tokens;
        bracketed!(enumerant_attr_tokens in input);
        let enumerant_name: Ident = enumerant_attr_tokens.parse()?;
        if enumerant_name != "enumerant" {
            return Err(Error::new_spanned(
                enumerant_name,
                "expected `#[enumerant = ...]` attribute",
            ));
        }
        enumerant_attr_tokens.parse::<Token!(=)>()?;
        let enumerant: Ident = enumerant_attr_tokens.parse()?;
        input.parse::<Token!(fn)>()?;
        let fn_name: Ident = input.parse()?;
        let inputs_tokens;
        parenthesized!(inputs_tokens in input);
        let inputs = inputs_tokens.parse_terminated(InstructionInput::parse)?;
        input.parse::<Token!(->)>()?;
        let outputs_tokens;
        parenthesized!(outputs_tokens in input);
        let outputs = outputs_tokens.parse_terminated(InstructionOutput::parse)?;
        let body_tokens;
        braced!(body_tokens in input);
        let instruction_name: LitStr = body_tokens.parse()?;
        Ok(Self {
            enumerant,
            fn_name,
            inputs,
            outputs,
            instruction_name,
        })
    }
}

impl Instruction {
    fn map_input_registers(&self) -> syn::Result<Vec<TokenStream>> {
        let mut retval = Vec::new();
        for input in &self.inputs {
            match input {
                InstructionInput::Ra(_) => retval.push(quote! {InstructionInputRegister::Ra}),
                InstructionInput::Rb(_) => retval.push(quote! {InstructionInputRegister::Rb}),
                InstructionInput::Rc(_) => retval.push(quote! {InstructionInputRegister::Rc}),
                InstructionInput::Carry(_) => retval.push(quote! {InstructionInputRegister::Carry}),
            }
        }
        Ok(retval)
    }
    fn to_native_fn_tokens(&self) -> syn::Result<TokenStream> {
        let Instruction {
            enumerant,
            fn_name,
            inputs,
            outputs,
            instruction_name,
        } = self;
        let mut asm = InlineAssembly::new(instruction_name.span());
        let mut before_asm = Vec::<TokenStream>::new();
        let mut after_asm = Vec::<TokenStream>::new();
        for output in &self.outputs {
            match output {
                InstructionOutput::Rt(span) => {
                    unimplemented!("InstructionOutput::Rt");
                }
                InstructionOutput::Carry(span) => {
                    unimplemented!("InstructionOutput::Carry");
                }
                InstructionOutput::Overflow(span) => {
                    unimplemented!("InstructionOutput::Overflow");
                }
                InstructionOutput::CR0(span) => {
                    unimplemented!("InstructionOutput::CR0");
                }
            }
        }
        for input in &self.inputs {
            match input {
                InstructionInput::Ra(span) => {
                    unimplemented!("InstructionInput::Ra");
                }
                InstructionInput::Rb(span) => {
                    unimplemented!("InstructionInput::Rb");
                }
                InstructionInput::Rc(span) => {
                    unimplemented!("InstructionInput::Rc");
                }
                InstructionInput::Carry(span) => {
                    unimplemented!("InstructionInput::Carry");
                }
            }
        }
        Ok(quote! {
            pub fn #fn_name(inputs: InstructionInput) -> InstructionResult {
                #![allow(unused_variables, unused_assignments)]
                let InstructionInput {
                    ra,
                    rb,
                    rc,
                    carry,
                } = inputs;
                let rt: u64;
                let xer: u64;
                let cr: u32;
                #(#before_asm)*
                unsafe {
                    #asm;
                }
                let mut retval = InstructionOutput::default();
                #(#after_asm)*
                retval
            }
        })
    }
}

#[derive(Debug)]
struct Instructions {
    instructions: Vec<Instruction>,
}

impl Instructions {
    fn to_tokens(&self) -> syn::Result<TokenStream> {
        let mut fn_names = Vec::new();
        let mut instr_enumerants = Vec::new();
        let mut get_native_fn_match_cases = Vec::new();
        let mut get_model_fn_match_cases = Vec::new();
        let mut get_used_input_registers_match_cases = Vec::new();
        let mut name_match_cases = Vec::new();
        let mut enumerants = Vec::new();
        let mut native_fn_tokens = Vec::new();
        for instruction in &self.instructions {
            let Instruction {
                enumerant,
                fn_name,
                inputs,
                outputs,
                instruction_name,
            } = instruction;
            fn_names.push(fn_name);
            enumerants.push(enumerant);
            instr_enumerants.push(quote! {
                #[serde(rename = #instruction_name)]
                #enumerant,
            });
            get_native_fn_match_cases.push(quote! {
                Self::#enumerant => native_instrs::#fn_name,
            });
            get_model_fn_match_cases.push(quote! {
                Self::#enumerant => instr_models::#fn_name,
            });
            let mapped_input_registers = instruction.map_input_registers()?;
            get_used_input_registers_match_cases.push(quote! {
                Self::#enumerant => &[#(#mapped_input_registers),*],
            });
            name_match_cases.push(quote! {
                Self::#enumerant => #instruction_name,
            });
            native_fn_tokens.push(instruction.to_native_fn_tokens()?);
        }
        Ok(quote! {
            #[cfg(feature = "python")]
            macro_rules! wrap_all_instr_fns {
                ($m:ident) => {
                    wrap_instr_fns! {
                        #![pymodule($m)]

                        #(fn #fn_names(inputs: InstructionInput) -> InstructionOutput;)*
                    }
                };
            }

            #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
            pub enum Instr {
                #(#instr_enumerants)*
            }

            impl Instr {
                #[cfg(feature = "native_instrs")]
                pub fn get_native_fn(self) -> fn(InstructionInput) -> InstructionOutput {
                    match self {
                        #(#get_native_fn_match_cases)*
                    }
                }
                pub fn get_model_fn(self) -> fn(InstructionInput) -> InstructionOutput {
                    match self {
                        #(#get_model_fn_match_cases)*
                    }
                }
                pub fn get_used_input_registers(self) -> &'static [InstructionInputRegister] {
                    match self {
                        #(#get_used_input_registers_match_cases)*
                    }
                }
                pub fn name(self) -> &'static str {
                    match self {
                        #(#name_match_cases)*
                    }
                }
                pub const VALUES: &'static [Self] = &[
                    #(Self::#enumerants,)*
                ];
            }

            #[cfg(feature = "native_instrs")]
            pub mod native_instrs {
                use super::*;

                #(#native_fn_tokens)*
            }
        })
    }
}

impl Parse for Instructions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut instructions = Vec::new();
        while !input.is_empty() {
            instructions.push(input.parse()?);
        }
        Ok(Self { instructions })
    }
}

#[proc_macro]
pub fn instructions(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Instructions);
    match input.to_tokens() {
        Ok(retval) => retval,
        Err(err) => err.to_compile_error(),
    }
    .into()
}
