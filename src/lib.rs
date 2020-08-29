// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

#![cfg_attr(feature = "native_instrs", feature(llvm_asm))]

#[cfg(all(feature = "native_instrs", not(target_arch = "powerpc64")))]
compile_error!("native_instrs feature requires target_arch to be powerpc64");

pub mod instr_models;
mod serde_hex;

use power_instruction_analyzer_proc_macro::instructions;
use serde::{Deserialize, Serialize};
use serde_plain::forward_display_to_serde;
use std::{
    cmp::Ordering,
    fmt,
    ops::{Index, IndexMut},
};

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    T::default() == *v
}

// powerpc bit numbers count from MSB to LSB
const fn get_xer_bit_mask(powerpc_bit_num: usize) -> u64 {
    (1 << 63) >> powerpc_bit_num
}

macro_rules! xer_subset {
    (
        $struct_vis:vis struct $struct_name:ident {
            $(
                #[bit($powerpc_bit_num:expr, $mask_name:ident)]
                $field_vis:vis $field_name:ident: bool,
            )+
        }
    ) => {
        #[derive(Default, Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
        $struct_vis struct $struct_name {
            $(
                $field_vis $field_name: bool,
            )+
        }

        impl $struct_name {
            $(
                $field_vis const $mask_name: u64 = get_xer_bit_mask($powerpc_bit_num);
            )+
            pub const fn from_xer(xer: u64) -> Self {
                Self {
                    $(
                        $field_name: (xer & Self::$mask_name) != 0,
                    )+
                }
            }
            pub const fn to_xer(self) -> u64 {
                let mut retval = 0u64;
                $(
                    if self.$field_name {
                        retval |= Self::$mask_name;
                    }
                )+
                retval
            }
        }
    };
}

xer_subset! {
    pub struct OverflowFlags {
        #[bit(32, XER_SO_MASK)]
        pub so: bool,
        #[bit(33, XER_OV_MASK)]
        pub ov: bool,
        #[bit(44, XER_OV32_MASK)]
        pub ov32: bool,
    }
}

impl OverflowFlags {
    pub const fn from_overflow(overflow: bool) -> Self {
        Self {
            so: overflow,
            ov: overflow,
            ov32: overflow,
        }
    }
}

xer_subset! {
    pub struct CarryFlags {
        #[bit(34, XER_CA_MASK)]
        pub ca: bool,
        #[bit(45, XER_CA32_MASK)]
        pub ca32: bool,
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
pub struct InstructionOutput {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_hex::SerdeHex"
    )]
    pub rt: Option<u64>,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub overflow: Option<OverflowFlags>,
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub carry: Option<CarryFlags>,
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

#[derive(Debug)]
pub struct MissingInstructionInput {
    pub input: InstructionInputRegister,
}

impl fmt::Display for MissingInstructionInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "missing instruction input: {}", self.input)
    }
}

impl std::error::Error for MissingInstructionInput {}

pub type InstructionResult = Result<InstructionOutput, MissingInstructionInput>;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub enum InstructionInputRegister {
    #[serde(rename = "ra")]
    Ra,
    #[serde(rename = "rb")]
    Rb,
    #[serde(rename = "rc")]
    Rc,
    #[serde(rename = "carry")]
    Carry,
}

forward_display_to_serde!(InstructionInputRegister);

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct InstructionInput {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_hex::SerdeHex"
    )]
    pub ra: Option<u64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_hex::SerdeHex"
    )]
    pub rb: Option<u64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_hex::SerdeHex"
    )]
    pub rc: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", flatten)]
    pub carry: Option<CarryFlags>,
}

macro_rules! impl_instr_try_get {
    (
        $(
            $vis:vis fn $fn:ident -> $return_type:ty { .$field:ident else $error_enum:ident }
        )+
    ) => {
        impl InstructionInput {
            $(
                $vis fn $fn(self) -> Result<$return_type, MissingInstructionInput> {
                    self.$field.ok_or(MissingInstructionInput {
                        input: InstructionInputRegister::$error_enum,
                    })
                }
            )+
        }
    };
}

impl_instr_try_get! {
    pub fn try_get_ra -> u64 {
        .ra else Ra
    }
    pub fn try_get_rb -> u64 {
        .rb else Rb
    }
    pub fn try_get_rc -> u64 {
        .rc else Rc
    }
    pub fn try_get_carry -> CarryFlags {
        .carry else Carry
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
    pub native_outputs: Option<InstructionOutput>,
    pub model_outputs: InstructionOutput,
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
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)* "$0"])
    };
    ([ra $($args:ident)*], [$($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)* "$3"])
    };
    ([rb $($args:ident)*], [$($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)* "$4"])
    };
    ([rc $($args:ident)*], [$($results:ident)*], [$($strings:literal)*]) => {
        map_instr_asm_args!([$($args)*], [$($results)*], [$($strings)* "$5"])
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
                carry,
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
                    : "=&b"(rt), "=&b"(xer), "=&b"(cr)
                    : "b"(ra), "b"(rb), "b"(rc), "b"(0u64), "b"(!0x8000_0000u64)
                    : "xer", "cr");
            }
            let mut retval = InstructionOutput::default();
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

                    $(fn $fn(inputs: InstructionInput) -> InstructionOutput;)*
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
            pub fn get_native_fn(self) -> fn(InstructionInput) -> InstructionOutput {
                match self {
                    $(
                        Self::$enumerant => native_instrs::$fn,
                    )+
                }
            }
            pub fn get_model_fn(self) -> fn(InstructionInput) -> InstructionOutput {
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

instructions! {
    // add
    #[enumerant = Add]
    fn add(Ra, Rb) -> (Rt) {
        "add"
    }
    #[enumerant = AddO]
    fn addo(Ra, Rb) -> (Rt, Overflow) {
        "addo"
    }
    #[enumerant = Add_]
    fn add_(Ra, Rb) -> (Rt, CR0) {
        "add."
    }
    #[enumerant = AddO_]
    fn addo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "addo."
    }

    // subf
    #[enumerant = SubF]
    fn subf(Ra, Rb) -> (Rt) {
        "subf"
    }
    #[enumerant = SubFO]
    fn subfo(Ra, Rb) -> (Rt, Overflow) {
        "subfo"
    }
    #[enumerant = SubF_]
    fn subf_(Ra, Rb) -> (Rt, CR0) {
        "subf."
    }
    #[enumerant = SubFO_]
    fn subfo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "subfo."
    }

    // divde
    #[enumerant = DivDE]
    fn divde(Ra, Rb) -> (Rt) {
        "divde"
    }
    #[enumerant = DivDEO]
    fn divdeo(Ra, Rb) -> (Rt, Overflow) {
        "divdeo"
    }
    #[enumerant = DivDE_]
    fn divde_(Ra, Rb) -> (Rt, CR0) {
        "divde."
    }
    #[enumerant = DivDEO_]
    fn divdeo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "divdeo."
    }

    // divdeu
    #[enumerant = DivDEU]
    fn divdeu(Ra, Rb) -> (Rt) {
        "divdeu"
    }
    #[enumerant = DivDEUO]
    fn divdeuo(Ra, Rb) -> (Rt, Overflow) {
        "divdeuo"
    }
    #[enumerant = DivDEU_]
    fn divdeu_(Ra, Rb) -> (Rt, CR0) {
        "divdeu."
    }
    #[enumerant = DivDEUO_]
    fn divdeuo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "divdeuo."
    }

    // divd
    #[enumerant = DivD]
    fn divd(Ra, Rb) -> (Rt) {
        "divd"
    }
    #[enumerant = DivDO]
    fn divdo(Ra, Rb) -> (Rt, Overflow) {
        "divdo"
    }
    #[enumerant = DivD_]
    fn divd_(Ra, Rb) -> (Rt, CR0) {
        "divd."
    }
    #[enumerant = DivDO_]
    fn divdo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "divdo."
    }

    // divdu
    #[enumerant = DivDU]
    fn divdu(Ra, Rb) -> (Rt) {
        "divdu"
    }
    #[enumerant = DivDUO]
    fn divduo(Ra, Rb) -> (Rt, Overflow) {
        "divduo"
    }
    #[enumerant = DivDU_]
    fn divdu_(Ra, Rb) -> (Rt, CR0) {
        "divdu."
    }
    #[enumerant = DivDUO_]
    fn divduo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "divduo."
    }

    // divwe
    #[enumerant = DivWE]
    fn divwe(Ra, Rb) -> (Rt) {
        "divwe"
    }
    #[enumerant = DivWEO]
    fn divweo(Ra, Rb) -> (Rt, Overflow) {
        "divweo"
    }
    #[enumerant = DivWE_]
    fn divwe_(Ra, Rb) -> (Rt, CR0) {
        "divwe."
    }
    #[enumerant = DivWEO_]
    fn divweo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "divweo."
    }

    // divweu
    #[enumerant = DivWEU]
    fn divweu(Ra, Rb) -> (Rt) {
        "divweu"
    }
    #[enumerant = DivWEUO]
    fn divweuo(Ra, Rb) -> (Rt, Overflow) {
        "divweuo"
    }
    #[enumerant = DivWEU_]
    fn divweu_(Ra, Rb) -> (Rt, CR0) {
        "divweu."
    }
    #[enumerant = DivWEUO_]
    fn divweuo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "divweuo."
    }

    // divw
    #[enumerant = DivW]
    fn divw(Ra, Rb) -> (Rt) {
        "divw"
    }
    #[enumerant = DivWO]
    fn divwo(Ra, Rb) -> (Rt, Overflow) {
        "divwo"
    }
    #[enumerant = DivW_]
    fn divw_(Ra, Rb) -> (Rt, CR0) {
        "divw."
    }
    #[enumerant = DivWO_]
    fn divwo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "divwo."
    }

    // divwu
    #[enumerant = DivWU]
    fn divwu(Ra, Rb) -> (Rt) {
        "divwu"
    }
    #[enumerant = DivWUO]
    fn divwuo(Ra, Rb) -> (Rt, Overflow) {
        "divwuo"
    }
    #[enumerant = DivWU_]
    fn divwu_(Ra, Rb) -> (Rt, CR0) {
        "divwu."
    }
    #[enumerant = DivWUO_]
    fn divwuo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "divwuo."
    }

    // mod*
    #[enumerant = ModSD]
    fn modsd(Ra, Rb) -> (Rt) {
        "modsd"
    }
    #[enumerant = ModUD]
    fn modud(Ra, Rb) -> (Rt) {
        "modud"
    }
    #[enumerant = ModSW]
    fn modsw(Ra, Rb) -> (Rt) {
        "modsw"
    }
    #[enumerant = ModUW]
    fn moduw(Ra, Rb) -> (Rt) {
        "moduw"
    }

    // mullw
    #[enumerant = MulLW]
    fn mullw(Ra, Rb) -> (Rt) {
        "mullw"
    }
    #[enumerant = MulLWO]
    fn mullwo(Ra, Rb) -> (Rt, Overflow) {
        "mullwo"
    }
    #[enumerant = MulLW_]
    fn mullw_(Ra, Rb) -> (Rt, CR0) {
        "mullw."
    }
    #[enumerant = MulLWO_]
    fn mullwo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "mullwo."
    }

    // mulhw
    #[enumerant = MulHW]
    fn mulhw(Ra, Rb) -> (Rt) {
        "mulhw"
    }
    #[enumerant = MulHW_]
    fn mulhw_(Ra, Rb) -> (Rt, CR0) {
        "mulhw."
    }

    // mulhwu
    #[enumerant = MulHWU]
    fn mulhwu(Ra, Rb) -> (Rt) {
        "mulhwu"
    }
    #[enumerant = MulHWU_]
    fn mulhwu_(Ra, Rb) -> (Rt, CR0) {
        "mulhwu."
    }

    // mulld
    #[enumerant = MulLD]
    fn mulld(Ra, Rb) -> (Rt) {
        "mulld"
    }
    #[enumerant = MulLDO]
    fn mulldo(Ra, Rb) -> (Rt, Overflow) {
        "mulldo"
    }
    #[enumerant = MulLD_]
    fn mulld_(Ra, Rb) -> (Rt, CR0) {
        "mulld."
    }
    #[enumerant = MulLDO_]
    fn mulldo_(Ra, Rb) -> (Rt, Overflow, CR0) {
        "mulldo."
    }

    // mulhd
    #[enumerant = MulHD]
    fn mulhd(Ra, Rb) -> (Rt) {
        "mulhd"
    }
    #[enumerant = MulHD_]
    fn mulhd_(Ra, Rb) -> (Rt, CR0) {
        "mulhd."
    }

    // mulhdu
    #[enumerant = MulHDU]
    fn mulhdu(Ra, Rb) -> (Rt) {
        "mulhdu"
    }
    #[enumerant = MulHDU_]
    fn mulhdu_(Ra, Rb) -> (Rt, CR0) {
        "mulhdu."
    }

    // madd*
    #[enumerant = MAddHD]
    fn maddhd(Ra, Rb, Rc) -> (Rt) {
        "maddhd"
    }
    #[enumerant = MAddHDU]
    fn maddhdu(Ra, Rb, Rc) -> (Rt) {
        "maddhdu"
    }
    #[enumerant = MAddLD]
    fn maddld(Ra, Rb, Rc) -> (Rt) {
        "maddld"
    }
}

// must be after instrs macro call since it uses a macro definition
mod python;
