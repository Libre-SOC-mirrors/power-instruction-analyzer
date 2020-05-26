// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

#![feature(llvm_asm)]

#[derive(Copy, Clone, Debug)]
struct TestDivResult {
    result: u64,
    overflow: bool,
    overflow32: bool,
}

impl TestDivResult {
    fn from_result_xer(result: u64, xer: u64) -> Self {
        TestDivResult {
            result,
            overflow: (xer & 0x4000_0000) != 0,
            overflow32: (xer & 0x8_0000) != 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct TestDivInput {
    divisor: u64,
    dividend: u64,
    result_prev: u64,
}

macro_rules! make_div_fn {
    ($name:ident) => {
        impl TestDivInput {
            fn $name(self) -> TestDivResult {
                let Self {
                    divisor,
                    dividend,
                    result_prev,
                } = self;
                let result: u64;
                let xer: u64;
                unsafe {
                    llvm_asm!(
                        concat!(
                            stringify!($name),
                            " $0, $3, $4\n",
                            "mfxer $1"
                        )
                        : "=&r"(result), "=&r"(xer)
                        : "0"(result_prev), "r"(dividend), "r"(divisor)
                        : "xer");
                }
                TestDivResult::from_result_xer(result, xer)
            }
        }
    };
}

make_div_fn!(divdo);

fn main() {
    let inputs = TestDivInput {
        divisor: 0,
        dividend: 0,
        result_prev: 0x123456789ABCDEF,
    };
    dbg!(inputs);
    let outputs = inputs.divdo();
    dbg!(outputs);
}
