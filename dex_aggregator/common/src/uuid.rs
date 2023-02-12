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

use core::{fmt, str::FromStr};
use ink_prelude::string::String;
use scale::{Decode, Encode};
use uuid;

use crate::{utils::general_utils::slice_to_hex_string, PublicError, Result};

#[derive(Decode, Encode, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct Uuid(uuid::Bytes);

impl Uuid {
    pub fn new(val: uuid::Bytes) -> Self {
        Self { 0: val }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        let inner = uuid::Uuid::from_str(s).map_err(|_| PublicError::FormatNotAllowed)?;
        Ok(Self {
            0: inner.into_bytes(),
        })
    }

    pub fn to_hex_string(&self) -> String {
        slice_to_hex_string(&self.0)
    }
}

impl fmt::Debug for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Uuid({})", slice_to_hex_string(&self.0))
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Uuid({})", slice_to_hex_string(&self.0))
    }
}
