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
use pink_extension::chain_extension::{signing, SigType};
use scale::Encode;
use sp_core::sr25519;
use sp_runtime::{MultiAddress, MultiSignature};

use privadex_common::signature_scheme::SignatureScheme;

pub struct ExtrinsicSigConfig<AccountId> {
    pub sig_scheme: SignatureScheme,
    pub signer: AccountId,
    pub privkey: Vec<u8>,
}

impl<AccountId> ExtrinsicSigConfig<AccountId>
where
    AccountId: Copy + Encode,
{
    /// Do NOT call `encode` on the results of get_encoded_*() because it is already encoded

    pub fn get_encoded_signer(&self) -> Vec<u8> {
        match self.sig_scheme {
            SignatureScheme::Ethereum => self.signer.encode(),
            SignatureScheme::Sr25519 => MultiAddress::<AccountId, u32>::Id(self.signer).encode(),
        }
    }

    pub fn get_encoded_signature(&self, encoded_data: Vec<u8>) -> Vec<u8> {
        let payload = if encoded_data.len() > 256 {
            sp_core_hashing::blake2_256(&encoded_data).to_vec()
        } else {
            encoded_data
        };

        match self.sig_scheme {
            // Use Keccak-256 hasher instead of Blake2-256 (which is the ECDSA default)
            SignatureScheme::Ethereum => {
                signing::ecdsa_sign_prehashed(&self.privkey, sp_core_hashing::keccak_256(&payload))
                    .to_vec()
            }
            SignatureScheme::Sr25519 => {
                let signature = signing::sign(&payload, &self.privkey, SigType::Sr25519);
                MultiSignature::from(
                    sr25519::Signature::try_from(signature.as_slice())
                        .expect("Expected 64-byte raw signature"),
                )
                .encode()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use ink_env::debug_println;

    use privadex_common::utils::general_utils::slice_to_hex_string;

    use super::*;

    #[test]
    fn verify_eth_msg_signature() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let secret_key = hex!("e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a"); // Alice
        let pubkey = signing::get_public_key(&secret_key, SigType::Ecdsa);
        let msg = "I verify that I submitted transaction 0x80ceab2d79fed91a042507bdf85142e846e4acdaaca4df4e86184b13a50c763c";
        // This is the signature policy used on Moonbeam (https://polkadot.js.org/apps/#/signing/verify)
        let signature: [u8; 65] = SignatureScheme::Ethereum
            .prefix_then_sign_msg(msg.as_bytes(), &secret_key)
            .try_into()
            .unwrap();
        debug_println!("Raw msg: {:?}", slice_to_hex_string(&msg.as_bytes()));
        debug_println!("Signature: {:?}", slice_to_hex_string(&signature));

        let verified =
            SignatureScheme::Ethereum.verify_unprefixed_msg(&pubkey, msg.as_bytes(), &signature);
        assert_eq!(verified, true);
    }

    #[test]
    fn verify_sr25519_msg_signature() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        let secret_key = hex!("e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a"); // Alice
        let pubkey = signing::get_public_key(&secret_key, SigType::Sr25519);
        let msg = "test message";
        let signature = signing::sign(msg.as_bytes(), &secret_key, SigType::Sr25519);
        debug_println!("Raw msg: {:?}", slice_to_hex_string(&msg.as_bytes()));
        debug_println!("Signature: {:?}", slice_to_hex_string(&signature));
        let verified = SignatureScheme::Sr25519.verify(&pubkey, msg.as_bytes(), &signature);
        assert_eq!(verified, true);
    }
}
