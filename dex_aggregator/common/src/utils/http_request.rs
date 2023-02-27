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

use ink_prelude::{format, string::String, vec, vec::Vec};
use pink_extension::http_post;
#[allow(unused_imports)]
use scale::Encode;

use crate::{PublicError, Result};

pub fn http_post_wrapper(url: &str, data: Vec<u8>) -> Result<Vec<u8>> {
    let content_length = format!("{}", data.len());
    let headers: Vec<(String, String)> = vec![
        ("Content-Type".into(), "application/json".into()),
        ("Content-Length".into(), content_length),
    ];

    let response = http_post!(url, data, headers);
    if response.body.len() > 4_000 {
        ink_env::debug_println!(
            "{}: total = {} bytes, body = {} bytes",
            url,
            response.encoded_size(),
            response.body.len()
        );
    }
    if response.status_code != 200 {
        return Err(PublicError::RequestFailed);
    }

    let body = response.body;
    Ok(body)
}
