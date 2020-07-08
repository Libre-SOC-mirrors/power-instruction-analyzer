use crate::{DivInput, DivResult, OverflowFlags};

pub fn divdeo(inputs: DivInput) -> DivResult {
    let dividend = i128::from(inputs.dividend as i64) << 64;
    let divisor = i128::from(inputs.divisor as i64);
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
    DivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divdeuo(inputs: DivInput) -> DivResult {
    let dividend = u128::from(inputs.dividend) << 64;
    let divisor = u128::from(inputs.divisor);
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
    DivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divdo(inputs: DivInput) -> DivResult {
    let dividend = inputs.dividend as i64;
    let divisor = inputs.divisor as i64;
    let overflow;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i64::min_value()) {
        result = 0;
        overflow = true;
    } else {
        result = (dividend / divisor) as u64;
        overflow = false;
    }
    DivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divduo(inputs: DivInput) -> DivResult {
    let dividend: u64 = inputs.dividend;
    let divisor: u64 = inputs.divisor;
    let overflow;
    let result;
    if divisor == 0 {
        result = 0;
        overflow = true;
    } else {
        result = dividend / divisor;
        overflow = false;
    }
    DivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divweo(inputs: DivInput) -> DivResult {
    let dividend = i64::from(inputs.dividend as i32) << 32;
    let divisor = i64::from(inputs.divisor as i32);
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
    DivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divweuo(inputs: DivInput) -> DivResult {
    let dividend = u64::from(inputs.dividend as u32) << 32;
    let divisor = u64::from(inputs.divisor as u32);
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
    DivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divwo(inputs: DivInput) -> DivResult {
    let dividend = inputs.dividend as i32;
    let divisor = inputs.divisor as i32;
    let overflow;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i32::min_value()) {
        result = 0;
        overflow = true;
    } else {
        result = (dividend / divisor) as u32 as u64;
        overflow = false;
    }
    DivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divwuo(inputs: DivInput) -> DivResult {
    let dividend = inputs.dividend as u32;
    let divisor = inputs.divisor as u32;
    let overflow;
    let result;
    if divisor == 0 {
        result = 0;
        overflow = true;
    } else {
        result = (dividend / divisor) as u64;
        overflow = false;
    }
    DivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn modsd(inputs: DivInput) -> DivResult {
    let dividend = inputs.dividend as i64;
    let divisor = inputs.divisor as i64;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i64::min_value()) {
        result = 0;
    } else {
        result = (dividend % divisor) as u64;
    }
    DivResult {
        result,
        overflow: None,
    }
}

pub fn modud(inputs: DivInput) -> DivResult {
    let dividend: u64 = inputs.dividend;
    let divisor: u64 = inputs.divisor;
    let result;
    if divisor == 0 {
        result = 0;
    } else {
        result = dividend % divisor;
    }
    DivResult {
        result,
        overflow: None,
    }
}

pub fn modsw(inputs: DivInput) -> DivResult {
    let dividend = inputs.dividend as i32;
    let divisor = inputs.divisor as i32;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i32::min_value()) {
        result = 0;
    } else {
        result = (dividend % divisor) as u64;
    }
    DivResult {
        result,
        overflow: None,
    }
}

pub fn moduw(inputs: DivInput) -> DivResult {
    let dividend = inputs.dividend as u32;
    let divisor = inputs.divisor as u32;
    let result;
    if divisor == 0 {
        result = 0;
    } else {
        result = (dividend % divisor) as u64;
    }
    DivResult {
        result,
        overflow: None,
    }
}
