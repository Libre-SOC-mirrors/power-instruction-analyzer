// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

#![cfg_attr(feature = "native_instrs", feature(llvm_asm))]

#[cfg(all(feature = "native_instrs", not(target_arch = "powerpc64")))]
compile_error!("native_instrs feature requires target_arch to be powerpc64");

pub mod instr_models;
mod python;
mod serde_hex;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverflowFlags {
    pub overflow: bool,
    pub overflow32: bool,
}

impl OverflowFlags {
    pub fn from_xer(xer: u64) -> Self {
        Self {
            overflow: (xer & 0x4000_0000) != 0,
            overflow32: (xer & 0x8_0000) != 0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DivResult {
    #[serde(with = "serde_hex::SerdeHex")]
    pub result: u64,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub overflow: Option<OverflowFlags>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct DivInput {
    #[serde(with = "serde_hex::SerdeHex")]
    pub dividend: u64,
    #[serde(with = "serde_hex::SerdeHex")]
    pub divisor: u64,
    #[serde(default, with = "serde_hex::SerdeHex")]
    pub result_prev: u64,
}

fn is_false(v: &bool) -> bool {
    !v
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TestDivCase {
    pub instr: DivInstr,
    #[serde(flatten)]
    pub inputs: DivInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_outputs: Option<DivResult>,
    pub model_outputs: DivResult,
    #[serde(default, skip_serializing_if = "is_false")]
    pub model_mismatch: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WholeTest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_div_cases: Vec<TestDivCase>,
    pub any_model_mismatch: bool,
}

macro_rules! make_div_functions {
    (
        #[div]
        {
            $($div_enum:ident = $div_fn:ident ($div_instr:literal),)+
        }
        #[rem]
        {
            $($rem_enum:ident = $rem_fn:ident ($rem_instr:literal),)+
        }
    ) => {
        #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
        pub enum DivInstr {
            $(
                #[serde(rename = $div_instr)]
                $div_enum,
            )+
            $(
                #[serde(rename = $rem_instr)]
                $rem_enum,
            )+
        }

        impl DivInstr {
            #[cfg(feature = "native_instrs")]
            pub fn get_native_fn(self) -> fn(DivInput) -> DivResult {
                match self {
                    $(
                        Self::$div_enum => native_instrs::$div_fn,
                    )+
                    $(
                        Self::$rem_enum => native_instrs::$rem_fn,
                    )+
                }
            }
            pub fn get_model_fn(self) -> fn(DivInput) -> DivResult {
                match self {
                    $(
                        Self::$div_enum => instr_models::$div_fn,
                    )+
                    $(
                        Self::$rem_enum => instr_models::$rem_fn,
                    )+
                }
            }
            pub fn name(self) -> &'static str {
                match self {
                    $(
                        Self::$div_enum => $div_instr,
                    )+
                    $(
                        Self::$rem_enum => $rem_instr,
                    )+
                }
            }
            pub const VALUES: &'static [Self] = &[
                $(
                    Self::$div_enum,
                )+
                $(
                    Self::$rem_enum,
                )+
            ];
        }

        #[cfg(feature = "native_instrs")]
        pub mod native_instrs {
            use super::*;

            $(
                pub fn $div_fn(inputs: DivInput) -> DivResult {
                    let DivInput {
                        dividend,
                        divisor,
                        result_prev,
                    } = inputs;
                    let result: u64;
                    let xer: u64;
                    unsafe {
                        llvm_asm!(
                            concat!(
                                $div_instr,
                                " $0, $3, $4\n",
                                "mfxer $1"
                            )
                            : "=&r"(result), "=&r"(xer)
                            : "0"(result_prev), "r"(dividend), "r"(divisor)
                            : "xer");
                    }
                    DivResult {
                        result,
                        overflow: Some(OverflowFlags::from_xer(xer)),
                    }
                }
            )+
            $(
                pub fn $rem_fn(inputs: DivInput) -> DivResult {
                    let DivInput {
                        dividend,
                        divisor,
                        result_prev,
                    } = inputs;
                    let result: u64;
                    unsafe {
                        llvm_asm!(
                            concat!(
                                $rem_instr,
                                " $0, $2, $3"
                            )
                            : "=&r"(result)
                            : "0"(result_prev), "r"(dividend), "r"(divisor));
                    }
                    DivResult {
                        result,
                        overflow: None,
                    }
                }
            )+
        }
    };
}

make_div_functions! {
    #[div]
    {
        DivDEO = divdeo("divdeo"),
        DivDEUO = divdeuo("divdeuo"),
        DivDO = divdo("divdo"),
        DivDUO = divduo("divduo"),
        DivWEO = divweo("divweo"),
        DivWEUO = divweuo("divweuo"),
        DivWO = divwo("divwo"),
        DivWUO = divwuo("divwuo"),
    }
    #[rem]
    {
        ModSD = modsd("modsd"),
        ModUD = modud("modud"),
        ModSW = modsw("modsw"),
        ModUW = moduw("moduw"),
    }
}
