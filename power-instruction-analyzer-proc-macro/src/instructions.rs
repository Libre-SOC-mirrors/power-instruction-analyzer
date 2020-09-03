// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

use crate::inline_assembly::{Assembly, AssemblyWithTextSpan};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use std::{collections::HashMap, fmt, hash::Hash};
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error, LitStr, Token,
};

trait InstructionArg: Clone + fmt::Debug + ToTokens + Parse {
    type Enumerant: Copy + Eq + Hash + fmt::Debug;
    fn enumerant(&self) -> Self::Enumerant;
    fn span(&self) -> &Span;
    fn name(&self) -> &'static str;
    fn into_ident(self) -> Ident;
}

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

        impl InstructionArg for $enum_name<Span> {
            type Enumerant = $enum_name<()>;
            fn enumerant(&self) -> Self::Enumerant {
                $enum_name::enumerant(self)
            }
            fn span(&self) -> &Span {
                match self {
                    $(
                        $enum_name::$enumerant(span) => span,
                    )*
                }
            }
            fn name(&self) -> &'static str {
                $enum_name::name(self)
            }
            fn into_ident(self) -> Ident {
                match self {
                    $(
                        $enum_name::$enumerant(span) => Ident::new(stringify!($enumerant), span),
                    )*
                }
            }
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
        Overflow,
    }
}

ident_enum! {
    #[parse_error_msg = "unknown instruction output"]
    enum InstructionOutput {
        Rt,
        Carry,
        Overflow,
        CR0,
        CR1,
        CR2,
        CR3,
        CR4,
        CR5,
        CR6,
        CR7,
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

fn check_duplicate_free<'a, T: InstructionArg + 'a>(
    args: impl IntoIterator<Item = &'a T>,
) -> syn::Result<()> {
    let mut seen_args = HashMap::new();
    for arg in args {
        if let Some(prev_arg) = seen_args.insert(arg.enumerant(), arg) {
            let mut error = Error::new(
                arg.span().clone(),
                format_args!(
                    "duplicate instruction argument: {}",
                    arg.clone().into_ident()
                ),
            );
            error.combine(Error::new(
                prev_arg.span().clone(),
                format_args!(
                    "duplicate instruction argument: {}",
                    prev_arg.clone().into_ident()
                ),
            ));
            return Err(error);
        }
    }
    Ok(())
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
        check_duplicate_free(&inputs)?;
        input.parse::<Token!(->)>()?;
        let outputs_tokens;
        parenthesized!(outputs_tokens in input);
        let outputs = outputs_tokens.parse_terminated(InstructionOutput::parse)?;
        check_duplicate_free(&outputs)?;
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
            retval.push(match input {
                InstructionInput::Ra(_) => quote! {InstructionInputRegister::Ra},
                InstructionInput::Rb(_) => quote! {InstructionInputRegister::Rb},
                InstructionInput::Rc(_) => quote! {InstructionInputRegister::Rc},
                InstructionInput::Carry(_) => quote! {InstructionInputRegister::Carry},
                InstructionInput::Overflow(_) => quote! {InstructionInputRegister::Overflow},
            });
        }
        Ok(retval)
    }
    fn to_native_fn_tokens(&self) -> syn::Result<TokenStream> {
        let Instruction {
            enumerant: _,
            fn_name,
            inputs,
            outputs,
            instruction_name,
        } = self;
        let asm_instr = Assembly::from(instruction_name.value());
        let mut asm_instr_args = Vec::new();
        let mut before_instr_asm_lines = Vec::<Assembly>::new();
        let mut after_instr_asm_lines = Vec::<Assembly>::new();
        let mut before_asm = Vec::<TokenStream>::new();
        let mut after_asm = Vec::<TokenStream>::new();
        let mut need_carry_output = false;
        let mut need_overflow_output = false;
        let mut need_cr_output = false;
        for output in outputs {
            match output {
                InstructionOutput::Rt(_) => {
                    before_asm.push(quote! {let rt: u64;});
                    asm_instr_args.push(assembly! {"$" output{"=&b"(rt)} });
                    after_asm.push(quote! {retval.rt = Some(rt);});
                }
                InstructionOutput::Carry(_) => {
                    need_carry_output = true;
                }
                InstructionOutput::Overflow(_) => {
                    need_overflow_output = true;
                }
                InstructionOutput::CR0(_) => {
                    need_cr_output = true;
                    after_asm.push(quote! {
                        retval.cr0 = Some(ConditionRegister::from_cr_field(cr, 0));
                    });
                }
                InstructionOutput::CR1(_) => {
                    need_cr_output = true;
                    after_asm.push(quote! {
                        retval.cr1 = Some(ConditionRegister::from_cr_field(cr, 1));
                    });
                }
                InstructionOutput::CR2(_) => {
                    need_cr_output = true;
                    after_asm.push(quote! {
                        retval.cr2 = Some(ConditionRegister::from_cr_field(cr, 2));
                    });
                }
                InstructionOutput::CR3(_) => {
                    need_cr_output = true;
                    after_asm.push(quote! {
                        retval.cr3 = Some(ConditionRegister::from_cr_field(cr, 3));
                    });
                }
                InstructionOutput::CR4(_) => {
                    need_cr_output = true;
                    after_asm.push(quote! {
                        retval.cr4 = Some(ConditionRegister::from_cr_field(cr, 4));
                    });
                }
                InstructionOutput::CR5(_) => {
                    need_cr_output = true;
                    after_asm.push(quote! {
                        retval.cr5 = Some(ConditionRegister::from_cr_field(cr, 5));
                    });
                }
                InstructionOutput::CR6(_) => {
                    need_cr_output = true;
                    after_asm.push(quote! {
                        retval.cr6 = Some(ConditionRegister::from_cr_field(cr, 6));
                    });
                }
                InstructionOutput::CR7(_) => {
                    need_cr_output = true;
                    after_asm.push(quote! {
                        retval.cr7 = Some(ConditionRegister::from_cr_field(cr, 7));
                    });
                }
            }
        }
        let mut need_carry_input = false;
        let mut need_overflow_input = false;
        for input in inputs {
            match input {
                InstructionInput::Ra(_) => {
                    before_asm.push(quote! {let ra: u64 = inputs.try_get_ra()?;});
                    asm_instr_args.push(assembly! {"$" input{"b"(ra)} });
                }
                InstructionInput::Rb(_) => {
                    before_asm.push(quote! {let rb: u64 = inputs.try_get_rb()?;});
                    asm_instr_args.push(assembly! {"$" input{"b"(rb)} });
                }
                InstructionInput::Rc(_) => {
                    before_asm.push(quote! {let rc: u64 = inputs.try_get_rc()?;});
                    asm_instr_args.push(assembly! {"$" input{"b"(rc)} });
                }
                InstructionInput::Carry(_) => {
                    need_carry_input = true;
                }
                InstructionInput::Overflow(_) => {
                    need_overflow_input = true;
                }
            }
        }
        if need_carry_input || need_carry_output || need_overflow_input || need_overflow_output {
            before_asm.push(quote! {
                let mut xer_in: u64 = 0;
                let mut xer_mask_in: u64 = !0;
            });
            if need_carry_input || need_carry_output {
                before_asm.push(quote! {
                    xer_mask_in &= !CarryFlags::XER_MASK;
                });
            }
            if need_overflow_input || need_overflow_output {
                before_asm.push(quote! {
                    xer_mask_in &= !OverflowFlags::XER_MASK;
                });
            }
            if need_carry_input {
                before_asm.push(quote! {
                    xer_in |= inputs.try_get_carry()?.to_xer();
                });
            }
            if need_overflow_input {
                before_asm.push(quote! {
                    xer_in |= inputs.try_get_overflow()?.to_xer();
                });
            }
            before_asm.push(quote! {
                let xer_out: u64;
            });
            let xer_out;
            before_instr_asm_lines.push(assembly! {
                "mfxer $" output(xer_out = {"=&b"(xer_out)})
            });
            before_instr_asm_lines.push(assembly! {
                "and $" (xer_out) ", $" (xer_out) ", $" input{"b"(xer_mask_in)}
            });
            before_instr_asm_lines.push(assembly! {
                "or $" (xer_out) ", $" (xer_out) ", $" input{"b"(xer_in)}
            });
            before_instr_asm_lines.push(assembly! {
                "mtxer $" (xer_out) clobber{"xer"}
            });
            after_instr_asm_lines.push(assembly! {
                "mfxer $" (xer_out)
            });
            if need_carry_output {
                after_asm.push(quote! {
                    retval.carry = Some(CarryFlags::from_xer(xer_out));
                });
            }
            if need_overflow_output {
                after_asm.push(quote! {
                    retval.overflow = Some(OverflowFlags::from_xer(xer_out));
                });
            }
        }
        if need_cr_output {
            before_asm.push(quote! {
                let cr: u32;
            });
            after_instr_asm_lines.push(assembly! {
                "mfcr $" output{"=&b"(cr)} clobber{"cr"}
            });
        }
        let mut final_asm = assembly! {};
        for i in before_instr_asm_lines {
            append_assembly! {final_asm; (i) "\n"};
        }
        append_assembly!(final_asm; (asm_instr));
        let mut separator = " ";
        for i in asm_instr_args {
            append_assembly!(final_asm; (separator) (i));
            separator = ", ";
        }
        for i in after_instr_asm_lines {
            append_assembly! {final_asm; "\n" (i)};
        }
        let asm = AssemblyWithTextSpan {
            asm: final_asm,
            text_span: instruction_name.span(),
        };
        Ok(quote! {
            pub fn #fn_name(inputs: InstructionInput) -> InstructionResult {
                #![allow(unused_variables, unused_assignments)]
                #(#before_asm)*
                unsafe {
                    #asm;
                }
                let mut retval = InstructionOutput::default();
                #(#after_asm)*
                Ok(retval)
            }
        })
    }
}

#[derive(Debug)]
pub(crate) struct Instructions {
    instructions: Vec<Instruction>,
}

impl Instructions {
    pub(crate) fn to_tokens(&self) -> syn::Result<TokenStream> {
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
                inputs: _,
                outputs: _,
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

                        #(fn #fn_names(inputs: InstructionInput) -> InstructionResult;)*
                    }
                };
            }

            #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
            pub enum Instr {
                #(#instr_enumerants)*
            }

            impl Instr {
                #[cfg(feature = "native_instrs")]
                pub fn get_native_fn(self) -> fn(InstructionInput) -> InstructionResult {
                    match self {
                        #(#get_native_fn_match_cases)*
                    }
                }
                pub fn get_model_fn(self) -> fn(InstructionInput) -> InstructionResult {
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
