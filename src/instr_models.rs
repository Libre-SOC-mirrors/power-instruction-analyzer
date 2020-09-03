use crate::{
    ConditionRegister, InstructionInput, InstructionOutput, InstructionResult,
    MissingInstructionInput, OverflowFlags,
};

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
