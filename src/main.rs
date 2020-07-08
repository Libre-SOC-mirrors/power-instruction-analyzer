// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

use power_instruction_analyzer::{DivInput, DivInstr, TestDivCase, WholeTest};

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
    for &instr in DivInstr::VALUES {
        for &dividend in TEST_VALUES {
            for &divisor in TEST_VALUES {
                let inputs = DivInput {
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
