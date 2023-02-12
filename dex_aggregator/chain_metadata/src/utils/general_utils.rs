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

use ink_prelude::vec::Vec;
use core::fmt::Write;

use crate::common::{PublicError, Result};

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
