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

use core::fmt::Write;
use ink_prelude::{
    string::{String, ToString},
    vec::Vec,
};
use primitive_types::{U128, U256};

use crate::{PublicError, Result};

pub fn hex_string_to_vec(s: &str) -> Result<Vec<u8>> {
    if "0x" != &s[..2] {
        return Err(PublicError::InvalidHex);
    }
    hex::decode(&s[2..]).map_err(|_| PublicError::InvalidHex)
}

pub fn slice_to_hex_string(v: &[u8]) -> String {
    let mut res = "0x".to_string();
    for a in v.iter() {
        write!(res, "{:02x}", a).expect("should create hex string");
    }
    res
}

/// This is guaranteed to work as expected (full precision) AS LONG AS the
/// result is less than u128::MAX. That is the caller's responsibility
/// It is thus ideal if numerator < denominator by definition
pub fn mul_ratio_u128(val: u128, numerator: u128, denominator: u128) -> u128 {
    let product: U256 = U128::from(val).full_mul(U128::from(numerator));
    (product / denominator).low_u128()
}

#[cfg(test)]
mod general_utils_tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn hex_string_to_vec_test() {
        assert_eq!(
            hex_string_to_vec("0x0102030405060708090a0b0c0d0e0fff").unwrap(),
            hex!("0102030405060708090a0b0c0d0e0fff")
        );
    }

    #[test]
    fn slice_to_hex_string_test() {
        assert_eq!(
            &slice_to_hex_string(&hex!("0102030405060708090a0b0c0d0e0fff")),
            "0x0102030405060708090a0b0c0d0e0fff"
        );
    }
}
