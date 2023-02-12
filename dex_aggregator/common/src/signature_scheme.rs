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

use ink_prelude::{format, vec::Vec};
use pink_extension::chain_extension::{signing, SigType};
use scale::{Decode, Encode};
use sp_core_hashing;

// Defines what algorithm is used to sign extrinsics
#[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum SignatureScheme {
    Ethereum,
    Sr25519,
}

impl SignatureScheme {
    pub fn prefix_then_sign_msg(&self, msg: &[u8], secret_key: &[u8]) -> Vec<u8> {
        self.sign(&self.prefix_msg(msg), secret_key)
    }

    pub fn sign(&self, msg: &[u8], secret_key: &[u8]) -> Vec<u8> {
        match self {
            SignatureScheme::Ethereum => {
                signing::ecdsa_sign_prehashed(secret_key, sp_core_hashing::keccak_256(msg)).to_vec()
            }
            SignatureScheme::Sr25519 => signing::sign(msg, secret_key, SigType::Sr25519),
        }
    }

    pub fn verify_unprefixed_msg(&self, pubkey: &[u8], msg: &[u8], signature: &[u8]) -> bool {
        let prefixed_msg = self.prefix_msg(msg);
        self.verify(pubkey, &prefixed_msg, signature)
    }

    pub fn verify(&self, pubkey: &[u8], msg: &[u8], signature: &[u8]) -> bool {
        match self {
            SignatureScheme::Ethereum => {
                if let (Ok(s), Ok(p)) = (signature.try_into(), pubkey.try_into()) {
                    signing::ecdsa_verify_prehashed(s, sp_core_hashing::keccak_256(msg), p)
                } else {
                    false
                }
            }
            SignatureScheme::Sr25519 => signing::verify(msg, pubkey, signature, SigType::Sr25519),
        }
    }

    pub fn prefix_msg(&self, msg: &[u8]) -> Vec<u8> {
        if self == &SignatureScheme::Ethereum {
            // https://github.com/ethereum/go-ethereum/issues/3731
            [
                format!("\x19Ethereum Signed Message:\n{}", msg.len()).as_bytes(),
                msg,
            ]
            .concat()
        } else {
            msg.to_vec()
        }
    }
}
