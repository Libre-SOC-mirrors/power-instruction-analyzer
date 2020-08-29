// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use std::fmt;
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
        todo!()
    }
    fn to_assembly_text(&self) -> syn::Result<String> {
        let mut retval = String::new();
        retval += "mfxer $1\n\
                   and $1, $1, $7\n\
                   mtxer $1\n";
        todo!("map_instr_asm_args!([$($args)*], [$($results)*], []),");
        retval += "\n\
                   mfxer $1\n\
                   mfcr $2\n";
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
        let assembly_text = self.to_assembly_text()?;
        let mut handle_inputs = Vec::<TokenStream>::new();
        unimplemented!("fill handle_inputs");
        let mut handle_outputs = Vec::<TokenStream>::new();
        unimplemented!(
            "fill handle_outputs\
                        map_instr_results!(rt, xer, cr, retval, [$($results)*]);"
        );
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
                #(#handle_inputs)*
                unsafe {
                    llvm_asm!(
                        #assembly_text
                        : "=&b"(rt), "=&b"(xer), "=&b"(cr)
                        : "b"(ra), "b"(rb), "b"(rc), "b"(0u64), "b"(!0x8000_0000u64)
                        : "xer", "cr");
                }
                let mut retval = InstructionOutput::default();
                #(#handle_outputs)*
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
