use crate::{OverflowFlags, TestDivInput, TestDivResult};

pub fn divdeo(inputs: TestDivInput) -> TestDivResult {
    let dividend = i128::from(inputs.dividend as i64) << 64;
    let divisor = i128::from(inputs.divisor as i64);
    let overflow;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i128::min_value()) {
        result = 0;
        overflow = true;
    } else {
        let result128 = dividend / divisor;
        result = result128 as u64;
        overflow = result128 as i64 as i128 != result128;
    }
    TestDivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divdeuo(inputs: TestDivInput) -> TestDivResult {
    let dividend = u128::from(inputs.dividend) << 64;
    let divisor = u128::from(inputs.divisor);
    let overflow;
    let result;
    if divisor == 0 {
        result = 0;
        overflow = true;
    } else {
        let resultu128 = dividend / divisor;
        result = resultu128 as u64;
        overflow = resultu128 > u128::from(u64::max_value());
    }
    TestDivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divdo(inputs: TestDivInput) -> TestDivResult {
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
    TestDivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divduo(inputs: TestDivInput) -> TestDivResult {
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
    TestDivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divweo(inputs: TestDivInput) -> TestDivResult {
    let dividend = i64::from(inputs.dividend as i32) << 32;
    let divisor = i64::from(inputs.divisor as i32);
    let overflow;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i64::min_value()) {
        result = 0;
        overflow = true;
    } else {
        let result64 = dividend / divisor;
        result = result64 as u32 as u64;
        overflow = result64 as i32 as i64 != result64;
    }
    TestDivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divweuo(inputs: TestDivInput) -> TestDivResult {
    let dividend = u64::from(inputs.dividend) << 32;
    let divisor = u64::from(inputs.divisor);
    let overflow;
    let result;
    if divisor == 0 {
        result = 0;
        overflow = true;
    } else {
        let resultu64 = dividend / divisor;
        result = resultu64 as u32 as u64;
        overflow = resultu64 > u64::from(u32::max_value());
    }
    TestDivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divwo(inputs: TestDivInput) -> TestDivResult {
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
    TestDivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn divwuo(inputs: TestDivInput) -> TestDivResult {
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
    TestDivResult {
        result,
        overflow: Some(OverflowFlags {
            overflow,
            overflow32: overflow,
        }),
    }
}

pub fn modsd(inputs: TestDivInput) -> TestDivResult {
    let dividend = inputs.dividend as i64;
    let divisor = inputs.divisor as i64;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i64::min_value()) {
        result = 0;
    } else {
        result = (dividend % divisor) as u64;
    }
    TestDivResult {
        result,
        overflow: None,
    }
}

pub fn modud(inputs: TestDivInput) -> TestDivResult {
    let dividend: u64 = inputs.dividend;
    let divisor: u64 = inputs.divisor;
    let result;
    if divisor == 0 {
        result = 0;
    } else {
        result = dividend % divisor;
    }
    TestDivResult {
        result,
        overflow: None,
    }
}

pub fn modsw(inputs: TestDivInput) -> TestDivResult {
    let dividend = inputs.dividend as i32;
    let divisor = inputs.divisor as i32;
    let result;
    if divisor == 0 || (divisor == -1 && dividend == i32::min_value()) {
        result = 0;
    } else {
        result = (dividend % divisor) as u32 as u64;
    }
    TestDivResult {
        result,
        overflow: None,
    }
}

pub fn moduw(inputs: TestDivInput) -> TestDivResult {
    let dividend = inputs.dividend as u32;
    let divisor = inputs.divisor as u32;
    let result;
    if divisor == 0 {
        result = 0;
    } else {
        result = (dividend % divisor) as u64;
    }
    TestDivResult {
        result,
        overflow: None,
    }
}
