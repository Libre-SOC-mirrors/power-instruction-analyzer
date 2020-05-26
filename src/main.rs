// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

#![feature(llvm_asm)]
use std::fmt;

#[derive(Copy, Clone, Debug)]
struct OverflowFlags {
    overflow: bool,
    overflow32: bool,
}

impl OverflowFlags {
    fn from_xer(xer: u64) -> Self {
        Self {
            overflow: (xer & 0x4000_0000) != 0,
            overflow32: (xer & 0x8_0000) != 0,
        }
    }
}

impl fmt::Display for OverflowFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self {
            overflow,
            overflow32,
        } = *self;
        write!(
            f,
            "OV:{overflow}, OV32:{overflow32}",
            overflow = overflow as i32,
            overflow32 = overflow32 as i32,
        )
    }
}

#[derive(Copy, Clone, Debug)]
struct TestDivResult {
    result: u64,
    overflow: Option<OverflowFlags>,
}

impl fmt::Display for TestDivResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { result, overflow } = *self;
        write!(f, "{:#X}", result)?;
        if let Some(overflow) = overflow {
            write!(f, ", {}", overflow)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
struct TestDivInput {
    dividend: u64,
    divisor: u64,
    result_prev: u64,
}

impl fmt::Display for TestDivInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self {
            dividend,
            divisor,
            result_prev,
        } = *self;
        write!(
            f,
            "{:#X} div {:#X} (result_prev:{:#X})",
            dividend, divisor, result_prev,
        )
    }
}

macro_rules! make_div_functions {
    (
        #[div]
        {
            $($div_name:ident;)+
        }
        #[rem]
        {
            $($rem_name:ident;)+
        }
    ) => {
        impl TestDivInput {
            $(
                #[inline(never)]
                pub fn $div_name(self) -> TestDivResult {
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
                                stringify!($div_name),
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
                #[inline(never)]
                pub fn $rem_name(self) -> TestDivResult {
                    let Self {
                        dividend,
                        divisor,
                        result_prev,
                    } = self;
                    let result: u64;
                    unsafe {
                        llvm_asm!(
                            concat!(
                                stringify!($rem_name),
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
            pub const FUNCTIONS: &'static [(fn(TestDivInput) -> TestDivResult, &'static str)] = &[
                $((Self::$div_name, stringify!($div_name)),)+
                $((Self::$rem_name, stringify!($rem_name)),)+
            ];
        }
    };
}

make_div_functions! {
    #[div]
    {divdeo; divdeuo; divdo; divduo; divweo; divweuo; divwo; divwuo;}
    #[rem]
    {modsd; modud; modsw; moduw;}
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
    for &(f, name) in TestDivInput::FUNCTIONS {
        for &dividend in TEST_VALUES {
            for &divisor in TEST_VALUES {
                let inputs = TestDivInput {
                    dividend,
                    divisor,
                    result_prev: 0xFECD_BA98_7654_3210,
                };
                let outputs = f(inputs);
                println!("{}: {} -> {}", name, inputs, outputs);
            }
        }
    }
}
