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

#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod fixed_point;
pub mod signature_scheme;
pub mod utils;
pub mod uuid;

use ss58_registry::Ss58AddressFormat;

#[derive(Debug, Eq, PartialEq)]
pub enum PublicError {
    BadBase58,
    BadLength,
    FormatNotAllowed,
    InvalidChecksum,
    InvalidHex,
    InvalidPrefix,
    RequestFailed,
    UnknownSs58AddressFormat(Ss58AddressFormat),
}
pub(crate) type Result<T> = core::result::Result<T, PublicError>;
