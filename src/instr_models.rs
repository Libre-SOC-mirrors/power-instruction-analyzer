use crate::{
    CarryFlags, ConditionRegister, InstructionInput, InstructionOutput, InstructionResult,
    MissingInstructionInput, OverflowFlags,
};
use std::convert::TryFrom;

fn propagate_so(
    mut overflow: OverflowFlags,
    inputs: InstructionInput,
) -> Result<OverflowFlags, MissingInstructionInput> {
    if inputs.try_get_overflow()?.so {
        overflow.so = true;
    }
    Ok(overflow)
}

macro_rules! create_instr_variants_ov_cr {
    ($fn:ident, $fno:ident, $fn_:ident, $fno_:ident, $iwidth:ident) => {
        pub fn $fn(mut inputs: InstructionInput) -> InstructionResult {
            inputs.overflow = Some(OverflowFlags::default());
            Ok(InstructionOutput {
                overflow: None,
                ..$fno(inputs)?
            })
        }
        pub fn $fn_(inputs: InstructionInput) -> InstructionResult {
            let mut retval = $fno_(inputs)?;
            let mut cr0 = retval.cr0.as_mut().expect("expected cr0 to be set");
            cr0.so = inputs.try_get_overflow()?.so;
            retval.overflow = None;
            Ok(retval)
        }
        pub fn $fno_(inputs: InstructionInput) -> InstructionResult {
            let mut retval = $fno(inputs)?;
            let result = retval.rt.expect("expected rt to be set");
            let so = retval.overflow.expect("expected overflow to be set").so;
            let cr0 = ConditionRegister::from_signed_int(result as $iwidth, so);
            retval.cr0 = Some(cr0);
            Ok(retval)
        }
    };
}

macro_rules! create_instr_variants_cr {
    ($fn:ident, $fn_:ident, $iwidth:ident) => {
        pub fn $fn_(inputs: InstructionInput) -> InstructionResult {
            let mut retval = $fn(inputs)?;
            let result = retval.rt.expect("expected rt to be set");
            let cr0 = ConditionRegister::from_signed_int(
                result as $iwidth,
                inputs.try_get_overflow()?.so,
            );
            retval.cr0 = Some(cr0);
            Ok(retval)
        }
    };
}

pub fn addi(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let immediate = inputs.try_get_immediate_s16()? as i64;
    let result = ra.wrapping_add(immediate) as u64;
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

pub fn addis(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let immediate = inputs.try_get_immediate_s16()? as i64;
    let result = ra.wrapping_add(immediate << 16) as u64;
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(add, addo, add_, addo_, i64);

pub fn addo(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let rb = inputs.try_get_rb()? as i64;
    let (result, ov) = ra.overflowing_add(rb);
    let result = result as u64;
    let ov32 = (ra as i32).overflowing_add(rb as i32).1;
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        ..InstructionOutput::default()
    })
}

create_instr_variants_cr!(addic, addic_, i64);

pub fn addic(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let immediate = inputs.try_get_immediate_s16()? as i64;
    let result = ra.wrapping_add(immediate) as u64;
    let ca = (ra as u64).overflowing_add(immediate as u64).1;
    let ca32 = (ra as u32).overflowing_add(immediate as u32).1;
    Ok(InstructionOutput {
        rt: Some(result),
        carry: Some(CarryFlags { ca, ca32 }),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(subf, subfo, subf_, subfo_, i64);

pub fn subfo(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let rb = inputs.try_get_rb()? as i64;
    let (result, ov) = rb.overflowing_sub(ra);
    let result = result as u64;
    let ov32 = (rb as i32).overflowing_sub(ra as i32).1;
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        ..InstructionOutput::default()
    })
}

pub fn subfic(inputs: InstructionInput) -> InstructionResult {
    let ra: u64 = inputs.try_get_ra()?;
    let immediate: u64 = inputs.try_get_immediate_s16()? as i64 as u64;
    let not_ra = !ra;
    let result = not_ra.wrapping_add(immediate).wrapping_add(1);
    let ca = not_ra
        .checked_add(immediate)
        .and_then(|v| v.checked_add(1))
        .is_none();
    let ca32 = (not_ra as u32)
        .checked_add(immediate as u32)
        .and_then(|v| v.checked_add(1))
        .is_none();
    Ok(InstructionOutput {
        rt: Some(result),
        carry: Some(CarryFlags { ca, ca32 }),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(addc, addco, addc_, addco_, i64);

pub fn addco(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let rb = inputs.try_get_rb()? as i64;
    let (result, ov) = ra.overflowing_add(rb);
    let result = result as u64;
    let ov32 = (ra as i32).overflowing_add(rb as i32).1;
    let ca = (ra as u64).overflowing_add(rb as u64).1;
    let ca32 = (ra as u32).overflowing_add(rb as u32).1;
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        carry: Some(CarryFlags { ca, ca32 }),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(subfc, subfco, subfc_, subfco_, i64);

pub fn subfco(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let rb = inputs.try_get_rb()? as i64;
    let (result, ov) = rb.overflowing_sub(ra);
    let result = result as u64;
    let ov32 = (rb as i32).overflowing_sub(ra as i32).1;
    let ca = !(rb as u64).overflowing_sub(ra as u64).1;
    let ca32 = !(rb as u32).overflowing_sub(ra as u32).1;
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        carry: Some(CarryFlags { ca, ca32 }),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(adde, addeo, adde_, addeo_, i64);

pub fn addeo(inputs: InstructionInput) -> InstructionResult {
    let ra: u64 = inputs.try_get_ra()?;
    let rb: u64 = inputs.try_get_rb()?;
    let carry_in = inputs.try_get_carry()?.ca;
    let result_i128 = ra as i64 as i128 + rb as i64 as i128 + carry_in as i128;
    let result_u128 = ra as u128 + rb as u128 + carry_in as u128;
    let result32_i128 = ra as i32 as i128 + rb as i32 as i128 + carry_in as i128;
    let result32_u128 = ra as u32 as u128 + rb as u32 as u128 + carry_in as u128;
    let result = result_u128 as u64;
    let ov = i64::try_from(result_i128).is_err();
    let ov32 = i32::try_from(result32_i128).is_err();
    let ca = u64::try_from(result_u128).is_err();
    let ca32 = u32::try_from(result32_u128).is_err();
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        carry: Some(CarryFlags { ca, ca32 }),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(subfe, subfeo, subfe_, subfeo_, i64);

pub fn subfeo(inputs: InstructionInput) -> InstructionResult {
    let ra: u64 = inputs.try_get_ra()?;
    let rb: u64 = inputs.try_get_rb()?;
    let carry_in = inputs.try_get_carry()?.ca;
    let not_ra = !ra;
    let result_i128 = not_ra as i64 as i128 + rb as i64 as i128 + carry_in as i128;
    let result_u128 = not_ra as u128 + rb as u128 + carry_in as u128;
    let result32_i128 = not_ra as i32 as i128 + rb as i32 as i128 + carry_in as i128;
    let result32_u128 = not_ra as u32 as u128 + rb as u32 as u128 + carry_in as u128;
    let result = result_u128 as u64;
    let ov = i64::try_from(result_i128).is_err();
    let ov32 = i32::try_from(result32_i128).is_err();
    let ca = u64::try_from(result_u128).is_err();
    let ca32 = u32::try_from(result32_u128).is_err();
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        carry: Some(CarryFlags { ca, ca32 }),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(addme, addmeo, addme_, addmeo_, i64);

pub fn addmeo(inputs: InstructionInput) -> InstructionResult {
    let ra: u64 = inputs.try_get_ra()?;
    let rb: u64 = !0;
    let carry_in = inputs.try_get_carry()?.ca;
    let result_i128 = ra as i64 as i128 + rb as i64 as i128 + carry_in as i128;
    let result_u128 = ra as u128 + rb as u128 + carry_in as u128;
    let result32_i128 = ra as i32 as i128 + rb as i32 as i128 + carry_in as i128;
    let result32_u128 = ra as u32 as u128 + rb as u32 as u128 + carry_in as u128;
    let result = result_u128 as u64;
    let ov = i64::try_from(result_i128).is_err();
    let ov32 = i32::try_from(result32_i128).is_err();
    let ca = u64::try_from(result_u128).is_err();
    let ca32 = u32::try_from(result32_u128).is_err();
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        carry: Some(CarryFlags { ca, ca32 }),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(subfme, subfmeo, subfme_, subfmeo_, i64);

pub fn subfmeo(inputs: InstructionInput) -> InstructionResult {
    let ra: u64 = inputs.try_get_ra()?;
    let rb: u64 = !0;
    let carry_in = inputs.try_get_carry()?.ca;
    let not_ra = !ra;
    let result_i128 = not_ra as i64 as i128 + rb as i64 as i128 + carry_in as i128;
    let result_u128 = not_ra as u128 + rb as u128 + carry_in as u128;
    let result32_i128 = not_ra as i32 as i128 + rb as i32 as i128 + carry_in as i128;
    let result32_u128 = not_ra as u32 as u128 + rb as u32 as u128 + carry_in as u128;
    let result = result_u128 as u64;
    let ov = i64::try_from(result_i128).is_err();
    let ov32 = i32::try_from(result32_i128).is_err();
    let ca = u64::try_from(result_u128).is_err();
    let ca32 = u32::try_from(result32_u128).is_err();
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        carry: Some(CarryFlags { ca, ca32 }),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(addze, addzeo, addze_, addzeo_, i64);

pub fn addzeo(inputs: InstructionInput) -> InstructionResult {
    let ra: u64 = inputs.try_get_ra()?;
    let carry_in = inputs.try_get_carry()?.ca;
    let result_i128 = ra as i64 as i128 + carry_in as i128;
    let result_u128 = ra as u128 + carry_in as u128;
    let result32_i128 = ra as i32 as i128 + carry_in as i128;
    let result32_u128 = ra as u32 as u128 + carry_in as u128;
    let result = result_u128 as u64;
    let ov = i64::try_from(result_i128).is_err();
    let ov32 = i32::try_from(result32_i128).is_err();
    let ca = u64::try_from(result_u128).is_err();
    let ca32 = u32::try_from(result32_u128).is_err();
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        carry: Some(CarryFlags { ca, ca32 }),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(subfze, subfzeo, subfze_, subfzeo_, i64);

pub fn subfzeo(inputs: InstructionInput) -> InstructionResult {
    let ra: u64 = inputs.try_get_ra()?;
    let carry_in = inputs.try_get_carry()?.ca;
    let not_ra = !ra;
    let result_i128 = not_ra as i64 as i128 + carry_in as i128;
    let result_u128 = not_ra as u128 + carry_in as u128;
    let result32_i128 = not_ra as i32 as i128 + carry_in as i128;
    let result32_u128 = not_ra as u32 as u128 + carry_in as u128;
    let result = result_u128 as u64;
    let ov = i64::try_from(result_i128).is_err();
    let ov32 = i32::try_from(result32_i128).is_err();
    let ca = u64::try_from(result_u128).is_err();
    let ca32 = u32::try_from(result32_u128).is_err();
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        carry: Some(CarryFlags { ca, ca32 }),
        ..InstructionOutput::default()
    })
}

pub fn addex(inputs: InstructionInput) -> InstructionResult {
    let ra: u64 = inputs.try_get_ra()?;
    let rb: u64 = inputs.try_get_rb()?;
    let OverflowFlags {
        ov: carry_in, so, ..
    } = inputs.try_get_overflow()?;
    let result_u128 = ra as u128 + rb as u128 + carry_in as u128;
    let result32_u128 = ra as u32 as u128 + rb as u32 as u128 + carry_in as u128;
    let result = result_u128 as u64;
    let carry = u64::try_from(result_u128).is_err();
    let carry32 = u32::try_from(result32_u128).is_err();
    Ok(InstructionOutput {
        rt: Some(result),
        // doesn't change `so` on purpose
        overflow: Some(OverflowFlags {
            so,
            ov: carry,
            ov32: carry32,
        }),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(neg, nego, neg_, nego_, i64);

pub fn nego(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let result = ra.wrapping_neg() as u64;
    let ov = ra.checked_neg().is_none();
    let ov32 = (ra as i32).checked_neg().is_none();
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(OverflowFlags { so: ov, ov, ov32 }, inputs)?),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(divde, divdeo, divde_, divdeo_, i64);

pub fn divdeo(inputs: InstructionInput) -> InstructionResult {
    let dividend = i128::from(inputs.try_get_ra()? as i64) << 64;
    let divisor = i128::from(inputs.try_get_rb()? as i64);
    let overflow;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i128::min_value()) {
        result = 0;
        overflow = true;
    } else {
        let result128 = dividend / divisor;
        if result128 as i64 as i128 != result128 {
            result = 0;
            overflow = true;
        } else {
            result = result128 as u64;
            overflow = false;
        }
    }
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(
            OverflowFlags::from_overflow(overflow),
            inputs,
        )?),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(divdeu, divdeuo, divdeu_, divdeuo_, i64);

pub fn divdeuo(inputs: InstructionInput) -> InstructionResult {
    let dividend = u128::from(inputs.try_get_ra()?) << 64;
    let divisor = u128::from(inputs.try_get_rb()?);
    let overflow;
    let result;
    if divisor == 0 {
        result = 0;
        overflow = true;
    } else {
        let resultu128 = dividend / divisor;
        if resultu128 > u128::from(u64::max_value()) {
            result = 0;
            overflow = true;
        } else {
            result = resultu128 as u64;
            overflow = false;
        }
    }
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(
            OverflowFlags::from_overflow(overflow),
            inputs,
        )?),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(divd, divdo, divd_, divdo_, i64);

pub fn divdo(inputs: InstructionInput) -> InstructionResult {
    let dividend = inputs.try_get_ra()? as i64;
    let divisor = inputs.try_get_rb()? as i64;
    let overflow;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i64::min_value()) {
        result = 0;
        overflow = true;
    } else {
        result = (dividend / divisor) as u64;
        overflow = false;
    }
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(
            OverflowFlags::from_overflow(overflow),
            inputs,
        )?),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(divdu, divduo, divdu_, divduo_, i64);

pub fn divduo(inputs: InstructionInput) -> InstructionResult {
    let dividend: u64 = inputs.try_get_ra()?;
    let divisor: u64 = inputs.try_get_rb()?;
    let overflow;
    let result;
    if divisor == 0 {
        result = 0;
        overflow = true;
    } else {
        result = dividend / divisor;
        overflow = false;
    }
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(
            OverflowFlags::from_overflow(overflow),
            inputs,
        )?),
        ..InstructionOutput::default()
    })
}

// ISA doesn't define compare results -- POWER9 apparently uses i64 instead of i32
create_instr_variants_ov_cr!(divwe, divweo, divwe_, divweo_, i64);

pub fn divweo(inputs: InstructionInput) -> InstructionResult {
    let dividend = i64::from(inputs.try_get_ra()? as i32) << 32;
    let divisor = i64::from(inputs.try_get_rb()? as i32);
    let overflow;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i64::min_value()) {
        result = 0;
        overflow = true;
    } else {
        let result64 = dividend / divisor;
        if result64 as i32 as i64 != result64 {
            result = 0;
            overflow = true;
        } else {
            result = result64 as u32 as u64;
            overflow = false;
        }
    }
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(
            OverflowFlags::from_overflow(overflow),
            inputs,
        )?),
        ..InstructionOutput::default()
    })
}

// ISA doesn't define compare results -- POWER9 apparently uses i64 instead of i32
create_instr_variants_ov_cr!(divweu, divweuo, divweu_, divweuo_, i64);

pub fn divweuo(inputs: InstructionInput) -> InstructionResult {
    let dividend = u64::from(inputs.try_get_ra()? as u32) << 32;
    let divisor = u64::from(inputs.try_get_rb()? as u32);
    let overflow;
    let result;
    if divisor == 0 {
        result = 0;
        overflow = true;
    } else {
        let resultu64 = dividend / divisor;
        if resultu64 > u64::from(u32::max_value()) {
            result = 0;
            overflow = true;
        } else {
            result = resultu64 as u32 as u64;
            overflow = false;
        }
    }
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(
            OverflowFlags::from_overflow(overflow),
            inputs,
        )?),
        ..InstructionOutput::default()
    })
}

// ISA doesn't define compare results -- POWER9 apparently uses i64 instead of i32
create_instr_variants_ov_cr!(divw, divwo, divw_, divwo_, i64);

pub fn divwo(inputs: InstructionInput) -> InstructionResult {
    let dividend = inputs.try_get_ra()? as i32;
    let divisor = inputs.try_get_rb()? as i32;
    let overflow;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i32::min_value()) {
        result = 0;
        overflow = true;
    } else {
        result = (dividend / divisor) as u32 as u64;
        overflow = false;
    }
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(
            OverflowFlags::from_overflow(overflow),
            inputs,
        )?),
        ..InstructionOutput::default()
    })
}

// ISA doesn't define compare results -- POWER9 apparently uses i64 instead of i32
create_instr_variants_ov_cr!(divwu, divwuo, divwu_, divwuo_, i64);

pub fn divwuo(inputs: InstructionInput) -> InstructionResult {
    let dividend = inputs.try_get_ra()? as u32;
    let divisor = inputs.try_get_rb()? as u32;
    let overflow;
    let result;
    if divisor == 0 {
        result = 0;
        overflow = true;
    } else {
        result = (dividend / divisor) as u64;
        overflow = false;
    }
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(
            OverflowFlags::from_overflow(overflow),
            inputs,
        )?),
        ..InstructionOutput::default()
    })
}

pub fn modsd(inputs: InstructionInput) -> InstructionResult {
    let dividend = inputs.try_get_ra()? as i64;
    let divisor = inputs.try_get_rb()? as i64;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i64::min_value()) {
        result = 0;
    } else {
        result = (dividend % divisor) as u64;
    }
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

pub fn modud(inputs: InstructionInput) -> InstructionResult {
    let dividend: u64 = inputs.try_get_ra()?;
    let divisor: u64 = inputs.try_get_rb()?;
    let result;
    if divisor == 0 {
        result = 0;
    } else {
        result = dividend % divisor;
    }
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

pub fn modsw(inputs: InstructionInput) -> InstructionResult {
    let dividend = inputs.try_get_ra()? as i32;
    let divisor = inputs.try_get_rb()? as i32;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i32::min_value()) {
        result = 0;
    } else {
        result = (dividend % divisor) as u64;
    }
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

pub fn moduw(inputs: InstructionInput) -> InstructionResult {
    let dividend = inputs.try_get_ra()? as u32;
    let divisor = inputs.try_get_rb()? as u32;
    let result;
    if divisor == 0 {
        result = 0;
    } else {
        result = (dividend % divisor) as u64;
    }
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

pub fn mulli(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let immediate = inputs.try_get_immediate_s16()? as i64;
    let result = ra.wrapping_mul(immediate) as u64;
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(mullw, mullwo, mullw_, mullwo_, i64);

pub fn mullwo(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i32 as i64;
    let rb = inputs.try_get_rb()? as i32 as i64;
    let result = ra.wrapping_mul(rb) as u64;
    let overflow = result as i32 as i64 != result as i64;
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(
            OverflowFlags::from_overflow(overflow),
            inputs,
        )?),
        ..InstructionOutput::default()
    })
}

create_instr_variants_cr!(mulhw, mulhw_, i32);

pub fn mulhw(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i32 as i64;
    let rb = inputs.try_get_rb()? as i32 as i64;
    let result = (ra * rb) >> 32;
    let mut result = result as u32 as u64;
    result |= result << 32;
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

create_instr_variants_cr!(mulhwu, mulhwu_, i32);

pub fn mulhwu(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as u32 as u64;
    let rb = inputs.try_get_rb()? as u32 as u64;
    let result = (ra * rb) >> 32;
    let mut result = result as u32 as u64;
    result |= result << 32;
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

create_instr_variants_ov_cr!(mulld, mulldo, mulld_, mulldo_, i64);

pub fn mulldo(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let rb = inputs.try_get_rb()? as i64;
    let result = ra.wrapping_mul(rb) as u64;
    let overflow = ra.checked_mul(rb).is_none();
    Ok(InstructionOutput {
        rt: Some(result),
        overflow: Some(propagate_so(
            OverflowFlags::from_overflow(overflow),
            inputs,
        )?),
        ..InstructionOutput::default()
    })
}

create_instr_variants_cr!(mulhd, mulhd_, i64);

pub fn mulhd(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64 as i128;
    let rb = inputs.try_get_rb()? as i64 as i128;
    let result = ((ra * rb) >> 64) as i64;
    let result = result as u64;
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

create_instr_variants_cr!(mulhdu, mulhdu_, i64);

pub fn mulhdu(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as u128;
    let rb = inputs.try_get_rb()? as u128;
    let result = ((ra * rb) >> 64) as u64;
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

pub fn maddhd(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64 as i128;
    let rb = inputs.try_get_rb()? as i64 as i128;
    let rc = inputs.try_get_rc()? as i64 as i128;
    let result = ((ra * rb + rc) >> 64) as u64;
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

pub fn maddhdu(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as u128;
    let rb = inputs.try_get_rb()? as u128;
    let rc = inputs.try_get_rc()? as u128;
    let result = ((ra * rb + rc) >> 64) as u64;
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

pub fn maddld(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let rb = inputs.try_get_rb()? as i64;
    let rc = inputs.try_get_rc()? as i64;
    let result = ra.wrapping_mul(rb).wrapping_add(rc) as u64;
    Ok(InstructionOutput {
        rt: Some(result),
        ..InstructionOutput::default()
    })
}

pub fn cmpdi(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let immediate = inputs.try_get_immediate_s16()? as i64;
    let so = inputs.try_get_overflow()?.so;
    let cr0 = ConditionRegister::from_ordering(ra.cmp(&immediate), so);
    Ok(InstructionOutput {
        cr0: Some(cr0),
        ..InstructionOutput::default()
    })
}

pub fn cmpwi(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i32;
    let immediate = inputs.try_get_immediate_s16()? as i32;
    let so = inputs.try_get_overflow()?.so;
    let cr0 = ConditionRegister::from_ordering(ra.cmp(&immediate), so);
    Ok(InstructionOutput {
        cr0: Some(cr0),
        ..InstructionOutput::default()
    })
}

pub fn cmpldi(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as u64;
    let immediate = inputs.try_get_immediate_u16()? as u64;
    let so = inputs.try_get_overflow()?.so;
    let cr0 = ConditionRegister::from_ordering(ra.cmp(&immediate), so);
    Ok(InstructionOutput {
        cr0: Some(cr0),
        ..InstructionOutput::default()
    })
}

pub fn cmplwi(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as u32;
    let immediate = inputs.try_get_immediate_u16()? as u32;
    let so = inputs.try_get_overflow()?.so;
    let cr0 = ConditionRegister::from_ordering(ra.cmp(&immediate), so);
    Ok(InstructionOutput {
        cr0: Some(cr0),
        ..InstructionOutput::default()
    })
}

pub fn cmpd(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i64;
    let rb = inputs.try_get_rb()? as i64;
    let so = inputs.try_get_overflow()?.so;
    let cr0 = ConditionRegister::from_ordering(ra.cmp(&rb), so);
    Ok(InstructionOutput {
        cr0: Some(cr0),
        ..InstructionOutput::default()
    })
}

pub fn cmpw(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as i32;
    let rb = inputs.try_get_rb()? as i32;
    let so = inputs.try_get_overflow()?.so;
    let cr0 = ConditionRegister::from_ordering(ra.cmp(&rb), so);
    Ok(InstructionOutput {
        cr0: Some(cr0),
        ..InstructionOutput::default()
    })
}

pub fn cmpld(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as u64;
    let rb = inputs.try_get_rb()? as u64;
    let so = inputs.try_get_overflow()?.so;
    let cr0 = ConditionRegister::from_ordering(ra.cmp(&rb), so);
    Ok(InstructionOutput {
        cr0: Some(cr0),
        ..InstructionOutput::default()
    })
}

pub fn cmplw(inputs: InstructionInput) -> InstructionResult {
    let ra = inputs.try_get_ra()? as u32;
    let rb = inputs.try_get_rb()? as u32;
    let so = inputs.try_get_overflow()?.so;
    let cr0 = ConditionRegister::from_ordering(ra.cmp(&rb), so);
    Ok(InstructionOutput {
        cr0: Some(cr0),
        ..InstructionOutput::default()
    })
}
