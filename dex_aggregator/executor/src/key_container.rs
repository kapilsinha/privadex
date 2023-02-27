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
use privadex_chain_metadata::common::{SecretKey, UniversalAddress};

#[derive(Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct KeyContainer(pub Vec<AddressKeyPair>);

#[derive(Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct AddressKeyPair {
    pub address: UniversalAddress,
    pub key: SecretKey,
}

impl KeyContainer {
    pub fn get_key(&self, address: &UniversalAddress) -> Option<&SecretKey> {
        for pair in self.0.iter() {
            if pair.address == *address {
                return Some(&pair.key);
            }
        }
        None
    }
}

#[cfg(test)]
mod key_container_tests {
    use hex_literal::hex;

    use privadex_chain_metadata::common::{EthAddress, SubstratePublicKey};

    use super::*;

    fn create_dummy_keycontainer() -> KeyContainer {
        KeyContainer {
            0: vec![
                AddressKeyPair {
                    address: UniversalAddress::Ethereum(EthAddress {
                        0: hex!("0102030405060708090a0b0c0d0e0f1011121314"),
                    }),
                    key: hex!("ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00"),
                },
                AddressKeyPair {
                    address: UniversalAddress::Substrate(SubstratePublicKey {
                        0: hex!("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"),
                    }),
                    key: hex!("11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee"),
                },
                AddressKeyPair {
                    address: UniversalAddress::Substrate(SubstratePublicKey {
                        0: hex!("ff0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"),
                    }),
                    key: hex!("22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd"),
                },
                AddressKeyPair {
                    address: UniversalAddress::Ethereum(EthAddress {
                        0: hex!("ff02030405060708090a0b0c0d0e0f1011121314"),
                    }),
                    key: hex!("cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33"),
                },
            ],
        }
    }

    #[test]
    fn test_get_key() {
        let key_container = create_dummy_keycontainer();
        assert_eq!(
            key_container
                .get_key(&UniversalAddress::Ethereum(EthAddress {
                    0: hex!("0102030405060708090a0b0c0d0e0f1011121314"),
                }))
                .expect("Key exists"),
            &hex!("ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00ff00")
        );
        assert_eq!(
            key_container
                .get_key(&UniversalAddress::Substrate(SubstratePublicKey {
                    0: hex!("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"),
                }))
                .expect("Key exists"),
            &hex!("11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee11ee")
        );
        assert_eq!(
            key_container
                .get_key(&UniversalAddress::Substrate(SubstratePublicKey {
                    0: hex!("ff0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"),
                }))
                .expect("Key exists"),
            &hex!("22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd22dd")
        );
        assert_eq!(
            key_container
                .get_key(&UniversalAddress::Ethereum(EthAddress {
                    0: hex!("ff02030405060708090a0b0c0d0e0f1011121314"),
                }))
                .expect("Key exists"),
            &hex!("cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33cc33")
        );
    }

    #[test]
    fn test_missing_key() {
        let key_container = create_dummy_keycontainer();
        assert!(key_container
            .get_key(&UniversalAddress::Ethereum(EthAddress {
                0: hex!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"),
            }))
            .is_none());
        assert!(key_container
            .get_key(&UniversalAddress::Substrate(SubstratePublicKey {
                0: hex!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"),
            }))
            .is_none());
    }
}
