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
use std::{cmp::Ordering, fmt};

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
            $struct_vis const XER_MASK: u64 = $(Self::$mask_name)|+;
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
    #[serde(rename = "overflow")]
    Overflow,
    #[serde(rename = "immediate_s16")]
    ImmediateS16,
    #[serde(rename = "immediate_u16")]
    ImmediateU16,
}

forward_display_to_serde!(InstructionInputRegister);

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_hex::SerdeHex"
    )]
    pub immediate: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none", flatten)]
    pub carry: Option<CarryFlags>,
    #[serde(default, skip_serializing_if = "Option::is_none", flatten)]
    pub overflow: Option<OverflowFlags>,
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
    pub fn try_get_overflow -> OverflowFlags {
        .overflow else Overflow
    }
}

impl InstructionInput {
    fn try_get_immediate(
        self,
        input: InstructionInputRegister,
    ) -> Result<u64, MissingInstructionInput> {
        self.immediate.ok_or(MissingInstructionInput { input })
    }
    pub fn try_get_immediate_u16(self) -> Result<u16, MissingInstructionInput> {
        Ok(self.try_get_immediate(InstructionInputRegister::ImmediateU16)? as u16)
    }
    pub fn try_get_immediate_s16(self) -> Result<i16, MissingInstructionInput> {
        Ok(self.try_get_immediate(InstructionInputRegister::ImmediateS16)? as i16)
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

instructions! {
    #[enumerant = AddI]
    fn addi(Ra, ImmediateS16) -> (Rt) {
        "addi"
    }

    #[enumerant = AddIS]
    fn addis(Ra, ImmediateS16) -> (Rt) {
        "addis"
    }

    // add
    #[enumerant = Add]
    fn add(Ra, Rb) -> (Rt) {
        "add"
    }
    #[enumerant = AddO]
    fn addo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "addo"
    }
    #[enumerant = Add_]
    fn add_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "add."
    }
    #[enumerant = AddO_]
    fn addo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "addo."
    }

    // addic
    #[enumerant = AddIC]
    fn addic(Ra, ImmediateS16) -> (Rt, Carry) {
        "addic"
    }
    #[enumerant = AddIC_]
    fn addic_(Ra, ImmediateS16, Overflow) -> (Rt, Carry, CR0) {
        "addic."
    }

    // subf
    #[enumerant = SubF]
    fn subf(Ra, Rb) -> (Rt) {
        "subf"
    }
    #[enumerant = SubFO]
    fn subfo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "subfo"
    }
    #[enumerant = SubF_]
    fn subf_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "subf."
    }
    #[enumerant = SubFO_]
    fn subfo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "subfo."
    }

    // addc
    #[enumerant = AddC]
    fn addc(Ra, Rb) -> (Rt, Carry) {
        "addc"
    }
    #[enumerant = AddCO]
    fn addco(Ra, Rb, Overflow) -> (Rt, Carry, Overflow) {
        "addco"
    }
    #[enumerant = AddC_]
    fn addc_(Ra, Rb, Overflow) -> (Rt, Carry, CR0) {
        "addc."
    }
    #[enumerant = AddCO_]
    fn addco_(Ra, Rb, Overflow) -> (Rt, Carry, Overflow, CR0) {
        "addco."
    }

    // subfc
    #[enumerant = SubFC]
    fn subfc(Ra, Rb) -> (Rt, Carry) {
        "subfc"
    }
    #[enumerant = SubFCO]
    fn subfco(Ra, Rb, Overflow) -> (Rt, Carry, Overflow) {
        "subfco"
    }
    #[enumerant = SubFC_]
    fn subfc_(Ra, Rb, Overflow) -> (Rt, Carry, CR0) {
        "subfc."
    }
    #[enumerant = SubFCO_]
    fn subfco_(Ra, Rb, Overflow) -> (Rt, Carry, Overflow, CR0) {
        "subfco."
    }

    // adde
    #[enumerant = AddE]
    fn adde(Ra, Rb, Carry) -> (Rt, Carry) {
        "adde"
    }
    #[enumerant = AddEO]
    fn addeo(Ra, Rb, Overflow, Carry) -> (Rt, Carry, Overflow) {
        "addeo"
    }
    #[enumerant = AddE_]
    fn adde_(Ra, Rb, Overflow, Carry) -> (Rt, Carry, CR0) {
        "adde."
    }
    #[enumerant = AddEO_]
    fn addeo_(Ra, Rb, Overflow, Carry) -> (Rt, Carry, Overflow, CR0) {
        "addeo."
    }

    // subfe
    #[enumerant = SubFE]
    fn subfe(Ra, Rb, Carry) -> (Rt, Carry) {
        "subfe"
    }
    #[enumerant = SubFEO]
    fn subfeo(Ra, Rb, Overflow, Carry) -> (Rt, Carry, Overflow) {
        "subfeo"
    }
    #[enumerant = SubFE_]
    fn subfe_(Ra, Rb, Overflow, Carry) -> (Rt, Carry, CR0) {
        "subfe."
    }
    #[enumerant = SubFEO_]
    fn subfeo_(Ra, Rb, Overflow, Carry) -> (Rt, Carry, Overflow, CR0) {
        "subfeo."
    }

    // addme
    #[enumerant = AddME]
    fn addme(Ra, Carry) -> (Rt, Carry) {
        "addme"
    }
    #[enumerant = AddMEO]
    fn addmeo(Ra, Overflow, Carry) -> (Rt, Carry, Overflow) {
        "addmeo"
    }
    #[enumerant = AddME_]
    fn addme_(Ra, Overflow, Carry) -> (Rt, Carry, CR0) {
        "addme."
    }
    #[enumerant = AddMEO_]
    fn addmeo_(Ra, Overflow, Carry) -> (Rt, Carry, Overflow, CR0) {
        "addmeo."
    }

    // subfme
    #[enumerant = SubFME]
    fn subfme(Ra, Carry) -> (Rt, Carry) {
        "subfme"
    }
    #[enumerant = SubFMEO]
    fn subfmeo(Ra, Overflow, Carry) -> (Rt, Carry, Overflow) {
        "subfmeo"
    }
    #[enumerant = SubFME_]
    fn subfme_(Ra, Overflow, Carry) -> (Rt, Carry, CR0) {
        "subfme."
    }
    #[enumerant = SubFMEO_]
    fn subfmeo_(Ra, Overflow, Carry) -> (Rt, Carry, Overflow, CR0) {
        "subfmeo."
    }

    // addze
    #[enumerant = AddZE]
    fn addze(Ra, Carry) -> (Rt, Carry) {
        "addze"
    }
    #[enumerant = AddZEO]
    fn addzeo(Ra, Overflow, Carry) -> (Rt, Carry, Overflow) {
        "addzeo"
    }
    #[enumerant = AddZE_]
    fn addze_(Ra, Overflow, Carry) -> (Rt, Carry, CR0) {
        "addze."
    }
    #[enumerant = AddZEO_]
    fn addzeo_(Ra, Overflow, Carry) -> (Rt, Carry, Overflow, CR0) {
        "addzeo."
    }

    // subfze
    #[enumerant = SubFZE]
    fn subfze(Ra, Carry) -> (Rt, Carry) {
        "subfze"
    }
    #[enumerant = SubFZEO]
    fn subfzeo(Ra, Overflow, Carry) -> (Rt, Carry, Overflow) {
        "subfzeo"
    }
    #[enumerant = SubFZE_]
    fn subfze_(Ra, Overflow, Carry) -> (Rt, Carry, CR0) {
        "subfze."
    }
    #[enumerant = SubFZEO_]
    fn subfzeo_(Ra, Overflow, Carry) -> (Rt, Carry, Overflow, CR0) {
        "subfzeo."
    }

    #[enumerant = AddEX]
    fn addex(Ra("r3"), Rb("r4"), Overflow) -> (Rt("r5"), Overflow) {
        // work around LLVM not supporting addex instruction:
        "addex" : ".long 0x7CA32154 # addex r5, r3, r4, 0"
    }

    // neg
    #[enumerant = Neg]
    fn neg(Ra) -> (Rt) {
        "neg"
    }
    #[enumerant = NegO]
    fn nego(Ra, Overflow) -> (Rt, Overflow) {
        "nego"
    }
    #[enumerant = Neg_]
    fn neg_(Ra, Overflow) -> (Rt, CR0) {
        "neg."
    }
    #[enumerant = NegO_]
    fn nego_(Ra, Overflow) -> (Rt, Overflow, CR0) {
        "nego."
    }

    // divde
    #[enumerant = DivDE]
    fn divde(Ra, Rb) -> (Rt) {
        "divde"
    }
    #[enumerant = DivDEO]
    fn divdeo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "divdeo"
    }
    #[enumerant = DivDE_]
    fn divde_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "divde."
    }
    #[enumerant = DivDEO_]
    fn divdeo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "divdeo."
    }

    // divdeu
    #[enumerant = DivDEU]
    fn divdeu(Ra, Rb) -> (Rt) {
        "divdeu"
    }
    #[enumerant = DivDEUO]
    fn divdeuo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "divdeuo"
    }
    #[enumerant = DivDEU_]
    fn divdeu_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "divdeu."
    }
    #[enumerant = DivDEUO_]
    fn divdeuo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "divdeuo."
    }

    // divd
    #[enumerant = DivD]
    fn divd(Ra, Rb) -> (Rt) {
        "divd"
    }
    #[enumerant = DivDO]
    fn divdo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "divdo"
    }
    #[enumerant = DivD_]
    fn divd_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "divd."
    }
    #[enumerant = DivDO_]
    fn divdo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "divdo."
    }

    // divdu
    #[enumerant = DivDU]
    fn divdu(Ra, Rb) -> (Rt) {
        "divdu"
    }
    #[enumerant = DivDUO]
    fn divduo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "divduo"
    }
    #[enumerant = DivDU_]
    fn divdu_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "divdu."
    }
    #[enumerant = DivDUO_]
    fn divduo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "divduo."
    }

    // divwe
    #[enumerant = DivWE]
    fn divwe(Ra, Rb) -> (Rt) {
        "divwe"
    }
    #[enumerant = DivWEO]
    fn divweo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "divweo"
    }
    #[enumerant = DivWE_]
    fn divwe_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "divwe."
    }
    #[enumerant = DivWEO_]
    fn divweo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "divweo."
    }

    // divweu
    #[enumerant = DivWEU]
    fn divweu(Ra, Rb) -> (Rt) {
        "divweu"
    }
    #[enumerant = DivWEUO]
    fn divweuo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "divweuo"
    }
    #[enumerant = DivWEU_]
    fn divweu_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "divweu."
    }
    #[enumerant = DivWEUO_]
    fn divweuo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "divweuo."
    }

    // divw
    #[enumerant = DivW]
    fn divw(Ra, Rb) -> (Rt) {
        "divw"
    }
    #[enumerant = DivWO]
    fn divwo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "divwo"
    }
    #[enumerant = DivW_]
    fn divw_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "divw."
    }
    #[enumerant = DivWO_]
    fn divwo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "divwo."
    }

    // divwu
    #[enumerant = DivWU]
    fn divwu(Ra, Rb) -> (Rt) {
        "divwu"
    }
    #[enumerant = DivWUO]
    fn divwuo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "divwuo"
    }
    #[enumerant = DivWU_]
    fn divwu_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "divwu."
    }
    #[enumerant = DivWUO_]
    fn divwuo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
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
    fn mullwo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "mullwo"
    }
    #[enumerant = MulLW_]
    fn mullw_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "mullw."
    }
    #[enumerant = MulLWO_]
    fn mullwo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "mullwo."
    }

    // mulhw
    #[enumerant = MulHW]
    fn mulhw(Ra, Rb) -> (Rt) {
        "mulhw"
    }
    #[enumerant = MulHW_]
    fn mulhw_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "mulhw."
    }

    // mulhwu
    #[enumerant = MulHWU]
    fn mulhwu(Ra, Rb) -> (Rt) {
        "mulhwu"
    }
    #[enumerant = MulHWU_]
    fn mulhwu_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "mulhwu."
    }

    // mulld
    #[enumerant = MulLD]
    fn mulld(Ra, Rb) -> (Rt) {
        "mulld"
    }
    #[enumerant = MulLDO]
    fn mulldo(Ra, Rb, Overflow) -> (Rt, Overflow) {
        "mulldo"
    }
    #[enumerant = MulLD_]
    fn mulld_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "mulld."
    }
    #[enumerant = MulLDO_]
    fn mulldo_(Ra, Rb, Overflow) -> (Rt, Overflow, CR0) {
        "mulldo."
    }

    // mulhd
    #[enumerant = MulHD]
    fn mulhd(Ra, Rb) -> (Rt) {
        "mulhd"
    }
    #[enumerant = MulHD_]
    fn mulhd_(Ra, Rb, Overflow) -> (Rt, CR0) {
        "mulhd."
    }

    // mulhdu
    #[enumerant = MulHDU]
    fn mulhdu(Ra, Rb) -> (Rt) {
        "mulhdu"
    }
    #[enumerant = MulHDU_]
    fn mulhdu_(Ra, Rb, Overflow) -> (Rt, CR0) {
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
