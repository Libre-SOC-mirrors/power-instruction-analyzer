// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

#![cfg_attr(feature = "native_instrs", feature(llvm_asm))]

mod instr_models;
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
pub struct TestDivResult {
    #[serde(with = "serde_hex::SerdeHex")]
    pub result: u64,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub overflow: Option<OverflowFlags>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TestDivInput {
    #[serde(with = "serde_hex::SerdeHex")]
    pub dividend: u64,
    #[serde(with = "serde_hex::SerdeHex")]
    pub divisor: u64,
    #[serde(with = "serde_hex::SerdeHex")]
    pub result_prev: u64,
}

fn is_false(v: &bool) -> bool {
    !v
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TestDivCase {
    pub instr: TestDivInstr,
    #[serde(flatten)]
    pub inputs: TestDivInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_outputs: Option<TestDivResult>,
    pub model_outputs: TestDivResult,
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
        pub enum TestDivInstr {
            $(
                #[serde(rename = $div_instr)]
                $div_enum,
            )+
            $(
                #[serde(rename = $rem_instr)]
                $rem_enum,
            )+
        }

        impl TestDivInstr {
            #[cfg(feature = "native_instrs")]
            pub fn get_native_fn(self) -> fn(TestDivInput) -> TestDivResult {
                match self {
                    $(
                        Self::$div_enum => native_instrs::$div_fn,
                    )+
                    $(
                        Self::$rem_enum => native_instrs::$rem_fn,
                    )+
                }
            }
            pub fn get_model_fn(self) -> fn(TestDivInput) -> TestDivResult {
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
        mod native_instrs {
            use super::*;

            $(
                pub fn $div_fn(inputs: TestDivInput) -> TestDivResult {
                    let TestDivInput {
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
                    TestDivResult {
                        result,
                        overflow: Some(OverflowFlags::from_xer(xer)),
                    }
                }
            )+
            $(
                pub fn $rem_fn(inputs: TestDivInput) -> TestDivResult {
                    let TestDivInput {
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
                    TestDivResult {
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

const TEST_VALUES: &[u64] = &[
    0x0,
    0x1,
    0x2,
    0xFFFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFE,
    0x7FFF_FFFF_FFFF_FFFF,
    0x8000_0000_0000_0000,
    0x1234_5678_0000_0000,
    0x1234_5678_8000_0000,
    0x1234_5678_FFFF_FFFF,
    0x1234_5678_7FFF_FFFF,
];

fn main() {
    let mut test_div_cases = Vec::new();
    let mut any_model_mismatch = false;
    for &instr in TestDivInstr::VALUES {
        for &dividend in TEST_VALUES {
            for &divisor in TEST_VALUES {
                let inputs = TestDivInput {
                    dividend,
                    divisor,
                    result_prev: 0xFECD_BA98_7654_3210,
                };
                let model_outputs = instr.get_model_fn()(inputs);
                #[cfg(feature = "native_instrs")]
                let native_outputs = Some(instr.get_native_fn()(inputs));
                #[cfg(not(feature = "native_instrs"))]
                let native_outputs = None;
                let model_mismatch = match native_outputs {
                    Some(native_outputs) if native_outputs != model_outputs => true,
                    _ => false,
                };
                any_model_mismatch |= model_mismatch;
                test_div_cases.push(TestDivCase {
                    instr,
                    inputs,
                    native_outputs,
                    model_outputs,
                    model_mismatch,
                });
            }
        }
    }
    let whole_test = WholeTest {
        test_div_cases,
        any_model_mismatch,
    };
    serde_json::to_writer_pretty(std::io::stdout().lock(), &whole_test).unwrap();
    println!();
}
