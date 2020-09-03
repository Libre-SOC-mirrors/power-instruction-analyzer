// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

use power_instruction_analyzer::{
    CarryFlags, Instr, InstructionInput, InstructionInputRegister, MissingInstructionInput,
    OverflowFlags, TestCase, WholeTest,
};

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

const BOOL_VALUES: &[bool] = &[false, true];

fn call_with_inputs(
    mut inputs: InstructionInput,
    input_registers: &[InstructionInputRegister],
    f: &mut impl FnMut(InstructionInput) -> Result<(), MissingInstructionInput>,
) -> Result<(), MissingInstructionInput> {
    if let Some((&input_register, input_registers)) = input_registers.split_first() {
        match input_register {
            InstructionInputRegister::Ra => {
                for &i in TEST_VALUES {
                    inputs.ra = Some(i);
                    call_with_inputs(inputs, input_registers, f)?;
                }
            }
            InstructionInputRegister::Rb => {
                for &i in TEST_VALUES {
                    inputs.rb = Some(i);
                    call_with_inputs(inputs, input_registers, f)?;
                }
            }
            InstructionInputRegister::Rc => {
                for &i in TEST_VALUES {
                    inputs.rc = Some(i);
                    call_with_inputs(inputs, input_registers, f)?;
                }
            }
            InstructionInputRegister::Carry => {
                for &ca in BOOL_VALUES {
                    for &ca32 in BOOL_VALUES {
                        inputs.carry = Some(CarryFlags { ca, ca32 });
                        call_with_inputs(inputs, input_registers, f)?;
                    }
                }
            }
            InstructionInputRegister::Overflow => {
                for &so in BOOL_VALUES {
                    for &ov in BOOL_VALUES {
                        for &ov32 in BOOL_VALUES {
                            inputs.overflow = Some(OverflowFlags { so, ov, ov32 });
                            call_with_inputs(inputs, input_registers, f)?;
                        }
                    }
                }
            }
        }
    } else {
        f(inputs)?;
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let mut test_cases = Vec::new();
    let mut any_model_mismatch = false;
    for &instr in Instr::VALUES {
        call_with_inputs(
            InstructionInput::default(),
            instr.get_used_input_registers(),
            &mut |inputs| -> Result<(), _> {
                let model_outputs = instr.get_model_fn()(inputs)?;
                #[cfg(feature = "native_instrs")]
                let native_outputs = Some(instr.get_native_fn()(inputs)?);
                #[cfg(not(feature = "native_instrs"))]
                let native_outputs = None;
                let model_mismatch = match native_outputs {
                    Some(native_outputs) if native_outputs != model_outputs => true,
                    _ => false,
                };
                any_model_mismatch |= model_mismatch;
                test_cases.push(TestCase {
                    instr,
                    inputs,
                    native_outputs,
                    model_outputs,
                    model_mismatch,
                });
                Ok(())
            },
        )
        .map_err(|err| format!("instruction {}: {}", instr.name(), err))?;
    }
    let whole_test = WholeTest {
        test_cases,
        any_model_mismatch,
    };
    serde_json::to_writer_pretty(std::io::stdout().lock(), &whole_test).unwrap();
    println!();
    Ok(())
}
