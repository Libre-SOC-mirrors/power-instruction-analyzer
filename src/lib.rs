// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

#![cfg_attr(feature = "native_instrs", feature(llvm_asm))]

#[cfg(all(feature = "native_instrs", not(target_arch = "powerpc64")))]
compile_error!("native_instrs feature requires target_arch to be powerpc64");

pub mod instr_models;
mod serde_hex;

use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    ops::{Index, IndexMut},
};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverflowFlags {
    pub so: bool,
    pub ov: bool,
    pub ov32: bool,
}

impl OverflowFlags {
    pub const fn from_xer(xer: u64) -> Self {
        Self {
            so: (xer & 0x8000_0000) != 0,
            ov: (xer & 0x4000_0000) != 0,
            ov32: (xer & 0x8_0000) != 0,
        }
    }
    pub const fn from_overflow(overflow: bool) -> Self {
        Self {
            so: overflow,
            ov: overflow,
            ov32: overflow,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConditionRegister {
    pub lt: bool,
    pub gt: bool,
    pub eq: bool,
    pub so: bool,
}

impl ConditionRegister {
    pub const fn from_4_bits(bits: u8) -> Self {
        // assert bits is 4-bits long
        // can switch to using assert! once rustc feature const_panic is stabilized
        [0; 0x10][bits as usize];

        Self {
            lt: (bits & 8) != 0,
            gt: (bits & 4) != 0,
            eq: (bits & 2) != 0,
            so: (bits & 1) != 0,
        }
    }
    pub const CR_FIELD_COUNT: usize = 8;
    pub const fn from_cr_field(cr: u32, field_index: usize) -> Self {
        // assert field_index is less than CR_FIELD_COUNT
        // can switch to using assert! once rustc feature const_panic is stabilized
        [0; Self::CR_FIELD_COUNT][field_index];

        let reversed_field_index = Self::CR_FIELD_COUNT - field_index - 1;
        let bits = (cr >> (4 * reversed_field_index)) & 0xF;
        Self::from_4_bits(bits as u8)
    }
    pub fn from_signed_int<T: Ord + Default>(value: T, so: bool) -> Self {
        let ordering = value.cmp(&T::default());
        Self {
            lt: ordering == Ordering::Less,
            gt: ordering == Ordering::Greater,
            eq: ordering == Ordering::Equal,
            so,
        }
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct InstructionResult {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_hex::SerdeHex"
    )]
    pub rt: Option<u64>,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub overflow: Option<OverflowFlags>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr0: Option<ConditionRegister>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr1: Option<ConditionRegister>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr2: Option<ConditionRegister>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr3: Option<ConditionRegister>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr4: Option<ConditionRegister>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr5: Option<ConditionRegister>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr6: Option<ConditionRegister>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cr7: Option<ConditionRegister>,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub enum InstructionInputRegister {
    #[serde(rename = "ra")]
    Ra,
    #[serde(rename = "rb")]
    Rb,
    #[serde(rename = "rc")]
    Rc,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct InstructionInput {
    #[serde(with = "serde_hex::SerdeHex")]
    pub ra: u64,
    #[serde(with = "serde_hex::SerdeHex")]
    pub rb: u64,
    #[serde(with = "serde_hex::SerdeHex")]
    pub rc: u64,
}

impl Index<InstructionInputRegister> for InstructionInput {
    type Output = u64;
    fn index(&self, index: InstructionInputRegister) -> &Self::Output {
        match index {
            InstructionInputRegister::Ra => &self.ra,
            InstructionInputRegister::Rb => &self.rb,
            InstructionInputRegister::Rc => &self.rc,
        }
    }
}

impl IndexMut<InstructionInputRegister> for InstructionInput {
    fn index_mut(&mut self, index: InstructionInputRegister) -> &mut Self::Output {
        match index {
            InstructionInputRegister::Ra => &mut self.ra,
            InstructionInputRegister::Rb => &mut self.rb,
            InstructionInputRegister::Rc => &mut self.rc,
        }
    }
}

fn is_false(v: &bool) -> bool {
    !v
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TestCase {
    pub instr: Instr,
    #[serde(flatten)]
    pub inputs: InstructionInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_outputs: Option<InstructionResult>,
    pub model_outputs: InstructionResult,
    #[serde(default, skip_serializing_if = "is_false")]
    pub model_mismatch: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WholeTest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_cases: Vec<TestCase>,
    pub any_model_mismatch: bool,
}

#[cfg(feature = "native_instrs")]
macro_rules! map_instr_asm_args {
    ([], [], []) => {
        ""
    };
    ([], [], [$string0:literal $($strings:literal)*]) => {
        concat!(" ", $string0, $(", ", $strings),*)
    };
    ([$($args:ident)*], [rt $($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], ["$0" $($strings)*])
    };
    ([ra $($args:ident)*], [$($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], ["$3" $($strings)*])
    };
    ([rb $($args:ident)*], [$($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], ["$4" $($strings)*])
    };
    ([rc $($args:ident)*], [$($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], ["$5" $($strings)*])
    };
    ([$($args:ident)*], [ov $($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)*])
    };
    ([$($args:ident)*], [cr0 $($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)*])
    };
    ([$($args:ident)*], [cr1 $($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)*])
    };
    ([$($args:ident)*], [cr2 $($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)*])
    };
    ([$($args:ident)*], [cr3 $($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)*])
    };
    ([$($args:ident)*], [cr4 $($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)*])
    };
    ([$($args:ident)*], [cr5 $($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)*])
    };
    ([$($args:ident)*], [cr6 $($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)*])
    };
    ([$($args:ident)*], [cr7 $($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)*])
    };
}

macro_rules! map_instr_input_registers {
    ([], [$($reg:expr,)*]) => {
        [$($reg,)*]
    };
    ([ra $($args:ident)*], [$($reg:expr,)*]) => {
        map_instr_input_registers!([$($args)*], [InstructionInputRegister::Ra, $($reg,)*])
    };
    ([rb $($args:ident)*], [$($reg:expr,)*]) => {
        map_instr_input_registers!([$($args)*], [InstructionInputRegister::Rb, $($reg,)*])
    };
    ([rc $($args:ident)*], [$($reg:expr,)*]) => {
        map_instr_input_registers!([$($args)*], [InstructionInputRegister::Rc, $($reg,)*])
    };
}

#[cfg(feature = "native_instrs")]
macro_rules! map_instr_results {
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, []) => {};
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, [rt $($args:ident)*]) => {
        $retval.rt = Some($rt);
        map_instr_results!($rt, $xer, $cr, $retval, [$($args)*]);
    };
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, [ov $($args:ident)*]) => {
        $retval.overflow = Some(OverflowFlags::from_xer($xer));
        map_instr_results!($rt, $xer, $cr, $retval, [$($args)*]);
    };
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, [cr0 $($args:ident)*]) => {
        $retval.cr0 = Some(ConditionRegister::from_cr_field($cr, 0));
        map_instr_results!($rt, $xer, $cr, $retval, [$($args)*]);
    };
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, [cr1 $($args:ident)*]) => {
        $retval.cr1 = Some(ConditionRegister::from_cr_field($cr, 1));
        map_instr_results!($rt, $xer, $cr, $retval, [$($args)*]);
    };
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, [cr2 $($args:ident)*]) => {
        $retval.cr2 = Some(ConditionRegister::from_cr_field($cr, 2));
        map_instr_results!($rt, $xer, $cr, $retval, [$($args)*]);
    };
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, [cr3 $($args:ident)*]) => {
        $retval.cr3 = Some(ConditionRegister::from_cr_field($cr, 3));
        map_instr_results!($rt, $xer, $cr, $retval, [$($args)*]);
    };
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, [cr4 $($args:ident)*]) => {
        $retval.cr4 = Some(ConditionRegister::from_cr_field($cr, 4));
        map_instr_results!($rt, $xer, $cr, $retval, [$($args)*]);
    };
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, [cr5 $($args:ident)*]) => {
        $retval.cr5 = Some(ConditionRegister::from_cr_field($cr, 5));
        map_instr_results!($rt, $xer, $cr, $retval, [$($args)*]);
    };
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, [cr6 $($args:ident)*]) => {
        $retval.cr6 = Some(ConditionRegister::from_cr_field($cr, 6));
        map_instr_results!($rt, $xer, $cr, $retval, [$($args)*]);
    };
    ($rt:ident, $xer:ident, $cr:ident, $retval:ident, [cr7 $($args:ident)*]) => {
        $retval.cr7 = Some(ConditionRegister::from_cr_field($cr, 7));
        map_instr_results!($rt, $xer, $cr, $retval, [$($args)*]);
    };
}

#[cfg(feature = "native_instrs")]
macro_rules! instr {
    (
        #[enumerant = $enumerant:ident]
        fn $fn:ident($($args:ident),*) -> ($($results:ident),*) {
            $instr:literal
        }
    ) => {
        pub fn $fn(inputs: InstructionInput) -> InstructionResult {
            #![allow(unused_variables, unused_assignments)]
            let InstructionInput {
                ra,
                rb,
                rc,
            } = inputs;
            let rt: u64;
            let xer: u64;
            let cr: u32;
            unsafe {
                llvm_asm!(
                    concat!(
                        "mfxer $1\n",
                        "and $1, $1, $7\n",
                        "mtxer $1\n",
                        $instr, " ",
                        map_instr_asm_args!([$($args)*], [$($results)*], []),
                        "\n",
                        "mfxer $1\n",
                        "mfcr $2\n",
                    )
                    : "=&r"(rt), "=&r"(xer), "=&r"(cr)
                    : "r"(ra), "r"(rb), "r"(rc), "r"(0u64), "r"(!0x8000_0000u64)
                    : "xer", "cr");
            }
            let mut retval = InstructionResult::default();
            map_instr_results!(rt, xer, cr, retval, [$($results)*]);
            retval
        }
    };
}

macro_rules! instrs {
    (
        $(
            #[enumerant = $enumerant:ident]
            fn $fn:ident($($args:ident),*) -> ($($results:ident),*) {
                $instr:literal
            }
        )+
    ) => {
        #[cfg(feature = "python")]
        macro_rules! wrap_all_instr_fns {
            ($m:ident) => {
                wrap_instr_fns! {
                    #![pymodule($m)]

                    $(fn $fn(inputs: InstructionInput) -> InstructionResult;)*
                }
            };
        }

        #[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
        pub enum Instr {
            $(
                #[serde(rename = $instr)]
                $enumerant,
            )+
        }

        impl Instr {
            #[cfg(feature = "native_instrs")]
            pub fn get_native_fn(self) -> fn(InstructionInput) -> InstructionResult {
                match self {
                    $(
                        Self::$enumerant => native_instrs::$fn,
                    )+
                }
            }
            pub fn get_model_fn(self) -> fn(InstructionInput) -> InstructionResult {
                match self {
                    $(
                        Self::$enumerant => instr_models::$fn,
                    )+
                }
            }
            pub fn get_used_input_registers(self) -> &'static [InstructionInputRegister] {
                match self {
                    $(
                        Self::$enumerant => &map_instr_input_registers!([$($args)*], []),
                    )+
                }
            }
            pub fn name(self) -> &'static str {
                match self {
                    $(
                        Self::$enumerant => $instr,
                    )+
                }
            }
            pub const VALUES: &'static [Self] = &[
                $(
                    Self::$enumerant,
                )+
            ];
        }

        #[cfg(feature = "native_instrs")]
        pub mod native_instrs {
            use super::*;

            $(
                instr! {
                    #[enumerant = $enumerant]
                    fn $fn($($args),*) -> ($($results),*) {
                        $instr
                    }
                }
            )+
        }
    };
}

instrs! {
    // divde
    #[enumerant = DivDE]
    fn divde(ra, rb) -> (rt) {
        "divde"
    }
    #[enumerant = DivDEO]
    fn divdeo(ra, rb) -> (rt, ov) {
        "divdeo"
    }
    #[enumerant = DivDE_]
    fn divde_(ra, rb) -> (rt, cr0) {
        "divde."
    }
    #[enumerant = DivDEO_]
    fn divdeo_(ra, rb) -> (rt, ov, cr0) {
        "divdeo."
    }

    // divdeu
    #[enumerant = DivDEU]
    fn divdeu(ra, rb) -> (rt) {
        "divdeu"
    }
    #[enumerant = DivDEUO]
    fn divdeuo(ra, rb) -> (rt, ov) {
        "divdeuo"
    }
    #[enumerant = DivDEU_]
    fn divdeu_(ra, rb) -> (rt, cr0) {
        "divdeu."
    }
    #[enumerant = DivDEUO_]
    fn divdeuo_(ra, rb) -> (rt, ov, cr0) {
        "divdeuo."
    }

    // divd
    #[enumerant = DivD]
    fn divd(ra, rb) -> (rt) {
        "divd"
    }
    #[enumerant = DivDO]
    fn divdo(ra, rb) -> (rt, ov) {
        "divdo"
    }
    #[enumerant = DivD_]
    fn divd_(ra, rb) -> (rt, cr0) {
        "divd."
    }
    #[enumerant = DivDO_]
    fn divdo_(ra, rb) -> (rt, ov, cr0) {
        "divdo."
    }

    // divdu
    #[enumerant = DivDU]
    fn divdu(ra, rb) -> (rt) {
        "divdu"
    }
    #[enumerant = DivDUO]
    fn divduo(ra, rb) -> (rt, ov) {
        "divduo"
    }
    #[enumerant = DivDU_]
    fn divdu_(ra, rb) -> (rt, cr0) {
        "divdu."
    }
    #[enumerant = DivDUO_]
    fn divduo_(ra, rb) -> (rt, ov, cr0) {
        "divduo."
    }

    // divwe
    #[enumerant = DivWE]
    fn divwe(ra, rb) -> (rt) {
        "divwe"
    }
    #[enumerant = DivWEO]
    fn divweo(ra, rb) -> (rt, ov) {
        "divweo"
    }
    #[enumerant = DivWE_]
    fn divwe_(ra, rb) -> (rt, cr0) {
        "divwe."
    }
    #[enumerant = DivWEO_]
    fn divweo_(ra, rb) -> (rt, ov, cr0) {
        "divweo."
    }

    // divweu
    #[enumerant = DivWEU]
    fn divweu(ra, rb) -> (rt) {
        "divweu"
    }
    #[enumerant = DivWEUO]
    fn divweuo(ra, rb) -> (rt, ov) {
        "divweuo"
    }
    #[enumerant = DivWEU_]
    fn divweu_(ra, rb) -> (rt, cr0) {
        "divweu."
    }
    #[enumerant = DivWEUO_]
    fn divweuo_(ra, rb) -> (rt, ov, cr0) {
        "divweuo."
    }

    // divw
    #[enumerant = DivW]
    fn divw(ra, rb) -> (rt) {
        "divw"
    }
    #[enumerant = DivWO]
    fn divwo(ra, rb) -> (rt, ov) {
        "divwo"
    }
    #[enumerant = DivW_]
    fn divw_(ra, rb) -> (rt, cr0) {
        "divw."
    }
    #[enumerant = DivWO_]
    fn divwo_(ra, rb) -> (rt, ov, cr0) {
        "divwo."
    }

    // divwu
    #[enumerant = DivWU]
    fn divwu(ra, rb) -> (rt) {
        "divwu"
    }
    #[enumerant = DivWUO]
    fn divwuo(ra, rb) -> (rt, ov) {
        "divwuo"
    }
    #[enumerant = DivWU_]
    fn divwu_(ra, rb) -> (rt, cr0) {
        "divwu."
    }
    #[enumerant = DivWUO_]
    fn divwuo_(ra, rb) -> (rt, ov, cr0) {
        "divwuo."
    }

    // mod*
    #[enumerant = ModSD]
    fn modsd(ra, rb) -> (rt) {
        "modsd"
    }
    #[enumerant = ModUD]
    fn modud(ra, rb) -> (rt) {
        "modud"
    }
    #[enumerant = ModSW]
    fn modsw(ra, rb) -> (rt) {
        "modsw"
    }
    #[enumerant = ModUW]
    fn moduw(ra, rb) -> (rt) {
        "moduw"
    }
}

// must be after instrs macro call since it uses a macro definition
mod python;
