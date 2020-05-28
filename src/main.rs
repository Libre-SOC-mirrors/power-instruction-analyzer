// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

#![feature(llvm_asm)]

mod serde_hex;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TestDivCase {
    pub instr: TestDivInstr,
    #[serde(flatten)]
    pub inputs: TestDivInput,
    #[serde(flatten)]
    pub outputs: TestDivResult,
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
            pub fn get_fn(self) -> fn(TestDivInput) -> TestDivResult {
                match self {
                    $(
                        Self::$div_enum => TestDivInput::$div_fn,
                    )+
                    $(
                        Self::$rem_enum => TestDivInput::$rem_fn,
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

        impl TestDivInput {
            $(
                pub fn $div_fn(self) -> TestDivResult {
                    let Self {
                        dividend,
                        divisor,
                        result_prev,
                    } = self;
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
                pub fn $rem_fn(self) -> TestDivResult {
                    let Self {
                        dividend,
                        divisor,
                        result_prev,
                    } = self;
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
        DivDE = divde("divdeo"),
        DivDEU = divdeu("divdeuo"),
        DivD = divd("divdo"),
        DivDU = divdu("divduo"),
        DivWE = divwe("divweo"),
        DivWEU = divweu("divweuo"),
        DivW = divw("divwo"),
        DivWU = divwu("divwuo"),
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
    0xFFFF_FFFF_FFFF_FFFF,
    0x7FFF_FFFF_FFFF_FFFF,
    0x8000_0000_0000_0000,
    0x1234_5678_0000_0000,
    0x1234_5678_8000_0000,
    0x1234_5678_FFFF_FFFF,
    0x1234_5678_7FFF_FFFF,
];

fn main() {
    let mut cases = Vec::new();
    for &instr in TestDivInstr::VALUES {
        for &dividend in TEST_VALUES {
            for &divisor in TEST_VALUES {
                let inputs = TestDivInput {
                    dividend,
                    divisor,
                    result_prev: 0xFECD_BA98_7654_3210,
                };
                let outputs = instr.get_fn()(inputs);
                cases.push(TestDivCase {
                    instr,
                    inputs,
                    outputs,
                });
            }
        }
    }
    serde_json::to_writer_pretty(std::io::stdout().lock(), &cases).unwrap();
    println!();
}
