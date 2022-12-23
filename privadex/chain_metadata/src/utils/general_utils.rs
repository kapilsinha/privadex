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
