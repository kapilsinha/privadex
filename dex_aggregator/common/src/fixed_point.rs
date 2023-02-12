/*
 * Copyright (C) 2023-present Kapil Sinha
 * Company: PrivaDEX
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the Server Side Public License, version 1,
 * as published by MongoDB, Inc.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * Server Side Public License for more details.
 *
 * You should have received a copy of the Server Side Public License
 * along with this program. If not, see
 * <http://www.mongodb.com/licensing/server-side-public-license>.
 */

use core::cmp::min;
use ink_prelude::string::String;
use primitive_types::{U128, U256};

// val = coef * 10^exp
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecimalFixedPoint {
    pub coef: u128,
    pub exp: i8,
}

impl DecimalFixedPoint {
    pub fn from_str_and_exp(num_str: &str, exp: u8) -> Self {
        let coef = string_num_shift_decimal_right_and_truncate(num_str, exp);
        Self {
            coef,
            exp: -(exp as i8),
        }
    }

    pub fn add_exp(&self, exp: i8) -> Self {
        Self {
            coef: self.coef,
            exp: self.exp + exp,
        }
    }

    pub fn val(&self) -> u128 {
        if self.exp >= 0 {
            self.coef * u128::pow(10, self.exp as u32)
        } else {
            self.coef / u128::pow(10, -self.exp as u32)
        }
    }

    pub fn mul_small(&self, other: &Self) -> Self {
        // We assume that self.coef * other.coef can be performed without overflow
        // This is true if we created both via from_str_and_exp with small exp
        // (such that the max value is less than 10^18)
        Self {
            coef: self.coef * other.coef,
            exp: self.exp + other.exp,
        }
    }

    pub fn mul_u128(&self, other: u128) -> u128 {
        let numerator: U256 = U128::full_mul(U128::from(self.coef), U128::from(other));
        if self.exp >= 0 {
            numerator.low_u128() * u128::pow(10, self.exp as u32)
        } else {
            (numerator / u128::pow(10, -self.exp as u32)).low_u128()
        }
    }

    pub fn u128_div(num: u128, denom: &Self) -> u128 {
        if denom.coef == 0 {
            u128::MAX
        } else if denom.exp >= 0 {
            num / denom.val()
        } else {
            (U128::full_mul(
                U128::from(num),
                U128::from(u128::pow(10, -denom.exp as u32)),
            ) / denom.coef)
                .low_u128()
        }
    }

    pub fn u128_mul_div(num: u128, mul_factor: &Self, div_factor: &Self) -> u128 {
        let exp = mul_factor.exp - div_factor.exp;
        if div_factor.coef == 0 {
            u128::MAX
        } else if exp >= 0 {
            let top = U256::full_mul(
                U128::full_mul(U128::from(num), U128::from(mul_factor.coef)),
                U256::from(u128::pow(10, exp as u32)),
            );
            (top / div_factor.coef).low_u128()
        } else {
            let top = U128::full_mul(U128::from(num), U128::from(mul_factor.coef));
            let bottom = U128::full_mul(
                U128::from(div_factor.coef),
                U128::from(u128::pow(10, -exp as u32)),
            );
            (top / bottom).low_u128()
        }
    }
}

// For example,
// Input: num = "0.00000012345", exp = +10
// Output: 1234
/// This will crash if you pass in a non-numerical string!
fn string_num_shift_decimal_right_and_truncate(num: &str, exp: u8) -> u128 {
    let mut shifted = String::from("");
    let mut num_shifts = exp as usize;
    if let Some(decimal_idx) = num.find(".") {
        shifted.push_str(&num[..decimal_idx]);
        let num_remaining = num.len() - decimal_idx - 1;
        let shifted_amt = min(num_remaining, exp as usize);
        let end_idx = decimal_idx + shifted_amt + 1;
        shifted.push_str(&num[decimal_idx + 1..end_idx]);
        num_shifts -= shifted_amt;
    } else {
        shifted.push_str(num);
    }
    shifted.push_str(&"0".repeat(num_shifts));
    shifted.parse().expect("String must be numerical")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_str_shift_decimal() {
        assert_eq!(
            string_num_shift_decimal_right_and_truncate("0.00000012345", 10),
            1234
        );
        assert_eq!(
            string_num_shift_decimal_right_and_truncate("0.00000012345", 5),
            0
        );
        assert_eq!(
            string_num_shift_decimal_right_and_truncate("0.00000012345", 15),
            123450000
        );
        assert_eq!(
            string_num_shift_decimal_right_and_truncate("0.00000012345", 25),
            1234500000000000000
        );
        assert_eq!(
            string_num_shift_decimal_right_and_truncate("12345", 3),
            12345000
        );
        assert_eq!(
            string_num_shift_decimal_right_and_truncate("12345.000000001", 10),
            123450000000010
        );
        assert_eq!(
            string_num_shift_decimal_right_and_truncate("12345.000000001", 8),
            1234500000000
        );
    }

    #[test]
    fn test_create_fixed_point() {
        let fixed = DecimalFixedPoint::from_str_and_exp("0.00000012345", 10);
        assert_eq!(
            fixed,
            DecimalFixedPoint {
                coef: 1234,
                exp: -10
            }
        );
    }

    #[test]
    fn test_mul_u128() {
        let fixed = DecimalFixedPoint::from_str_and_exp("0.00000012345", 10);
        assert_eq!(fixed.mul_u128(1_000_000_000_000), 123400);
        assert_eq!(fixed.mul_u128(100_000_000), 12);
        assert_eq!(
            fixed.add_exp(12).mul_u128(1_000_000_000),
            123_400_000_000_000
        )
    }

    #[test]
    fn test_u128_div() {
        let fixed = DecimalFixedPoint::from_str_and_exp("0.00000012345", 11);
        assert_eq!(DecimalFixedPoint::u128_div(24690, &fixed), 200_000_000_000);
        assert_eq!(DecimalFixedPoint::u128_div(24690000, &fixed.add_exp(15)), 0);
        assert_eq!(
            DecimalFixedPoint::u128_div(246900000, &fixed.add_exp(15)),
            2
        );
        assert_eq!(
            DecimalFixedPoint::u128_div(246900000000, &fixed.add_exp(15)),
            2000
        );
    }

    #[test]
    fn test_u128_mul_div() {
        {
            let fixed1 = DecimalFixedPoint::from_str_and_exp("0.00000024690", 11);
            let fixed2 = DecimalFixedPoint::from_str_and_exp("0.00000012345", 11);
            assert_eq!(
                DecimalFixedPoint::u128_mul_div(
                    1_000_000_000_000_000_000_000_000_000_000_000,
                    &fixed1,
                    &fixed2
                ),
                2_000_000_000_000_000_000_000_000_000_000_000
            );
        }
        {
            let fixed1 = DecimalFixedPoint::from_str_and_exp("0.00000024690", 10).add_exp(4);
            let fixed2 = DecimalFixedPoint::from_str_and_exp("0.00000012345", 11);
            assert_eq!(
                DecimalFixedPoint::u128_mul_div(
                    1_000_000_000_000_000_000_000_000_000_000,
                    &fixed1,
                    &fixed2
                ),
                20_000_000_000_000_000_000_000_000_000_000_000
            );
        }
        {
            let fixed1 = DecimalFixedPoint::from_str_and_exp("0.00000024690", 11);
            let fixed2 = DecimalFixedPoint::from_str_and_exp("0.00000012345", 11).add_exp(4);
            assert_eq!(
                DecimalFixedPoint::u128_mul_div(
                    1_000_000_000_000_000_000_000_000_000_000_000,
                    &fixed1,
                    &fixed2
                ),
                200_000_000_000_000_000_000_000_000_000
            );
        }
        {
            let fixed1 = DecimalFixedPoint::from_str_and_exp("24690000", 11);
            let fixed2 = DecimalFixedPoint::from_str_and_exp("0.00000012345", 11);
            assert_eq!(
                DecimalFixedPoint::u128_mul_div(
                    1_000_000_000_000_000_000_000_000,
                    &fixed1,
                    &fixed2
                ),
                200_000_000_000_000_000_000_000_000_000_000_000_000
            );
        }
    }
}
