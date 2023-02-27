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

use scale::{Decode, Encode};
use xcm::latest::{Junction, Junctions, MultiLocation, NetworkId};

use crate::chain_info::{AddressType, ChainInfo};
use crate::common::{
    Amount, PublicError, Result, UniversalAddress, UniversalChainId, UniversalTokenId,
};

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Bridge {
    Xcm(XCMBridge),
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct XCMBridge {
    pub src_token: UniversalTokenId,
    pub dest_token: UniversalTokenId,
    // From the source chain's perspective
    pub token_asset_multilocation: xcm::latest::MultiLocation,
    // Generates the MultiLocation for the destination wallet address
    pub dest_multilocation_template: WalletMultiLocationTemplate,
    pub estimated_bridge_fee_in_dest_chain_native_token: Amount,
}

trait DestMultiLocationGenerator<T> {
    // Moonbeam' xTokens.transferMultiasset extrinsic specifies the destination address
    // in a single MultiLocation
    fn get_full_dest_multilocation_raw(&self, field: T) -> Result<MultiLocation>;

    // Polkadot relay's xcmPallet.limitedReserveAssetTransfer and
    // Astar's polkadotXcm.limitedReserveAssetTransfer extrinsics specify the destination
    // chain and beneficiary as two separate MultiLocations
    // Note that Astar's polkadotXcm pallet has an identical interface to Polkadot relay's xcmPallet:
    // https://docs.astar.network/docs/xcm/building-with-xcm/client-applications/
    fn get_split_dest_and_beneficiary_multilocation_raw(
        &self,
        field: T,
    ) -> Result<(
        MultiLocation, /* dest */
        MultiLocation, /* beneficiary */
    )>;
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct WalletMultiLocationTemplate {
    template: MultiLocation,
}

impl WalletMultiLocationTemplate {
    pub fn get_full_dest_multilocation(&self, address: UniversalAddress) -> Result<MultiLocation> {
        match address {
            UniversalAddress::Ethereum(eth_addr) => {
                self.get_full_dest_multilocation_raw(eth_addr.0)
            }
            UniversalAddress::Substrate(substrate_addr) => {
                self.get_full_dest_multilocation_raw(substrate_addr.0)
            }
        }
    }
}

impl DestMultiLocationGenerator<[u8; 20]> for WalletMultiLocationTemplate {
    fn get_full_dest_multilocation_raw(&self, field: [u8; 20]) -> Result<MultiLocation> {
        let len = self.template.interior.len();
        if len == 0 {
            return Err(PublicError::InvalidMultiLocationLength);
        }
        let mut multilocation = self.template.clone();
        let last_junction = multilocation
            .interior
            .at_mut(len - 1)
            .ok_or(PublicError::InvalidMultiLocationLength)?;
        if let Junction::AccountKey20 { network: _, key } = last_junction {
            let _ = core::mem::replace(key, field);
            Ok(multilocation)
        } else {
            Err(PublicError::InvalidMultiLocationAddress)
        }
    }
    fn get_split_dest_and_beneficiary_multilocation_raw(
        &self,
        field: [u8; 20],
    ) -> Result<(
        MultiLocation, /* dest */
        MultiLocation, /* beneficiary */
    )> {
        let full_dest_multilocation = self.get_full_dest_multilocation_raw(field)?;
        split_into_dest_and_beneficiary(full_dest_multilocation)
    }
}

impl DestMultiLocationGenerator<[u8; 32]> for WalletMultiLocationTemplate {
    fn get_full_dest_multilocation_raw(&self, field: [u8; 32]) -> Result<MultiLocation> {
        let len = self.template.interior.len();
        if len == 0 {
            return Err(PublicError::InvalidMultiLocationLength);
        }
        let mut multilocation = self.template.clone();
        let last_junction = multilocation
            .interior
            .at_mut(len - 1)
            .ok_or(PublicError::InvalidMultiLocationLength)?;
        if let Junction::AccountId32 { network: _, id } = last_junction {
            let _ = core::mem::replace(id, field);
            Ok(multilocation)
        } else {
            Err(PublicError::InvalidMultiLocationAddress)
        }
    }

    fn get_split_dest_and_beneficiary_multilocation_raw(
        &self,
        field: [u8; 32],
    ) -> Result<(
        MultiLocation, /* dest */
        MultiLocation, /* beneficiary */
    )> {
        let full_dest_multilocation = self.get_full_dest_multilocation_raw(field)?;
        split_into_dest_and_beneficiary(full_dest_multilocation)
    }
}

pub fn split_into_dest_and_beneficiary(
    full_dest_multilocation: MultiLocation,
) -> Result<(MultiLocation, MultiLocation)> {
    // For limitedReserveAssetTransfers, I have only seen junctions of the form
    // X1: (AccountId) for parachain to relay chain or X2: (Parachain, AccountKey/ID) for others.
    // For safety I will require junctions of length 1 or 2
    if full_dest_multilocation.interior.len() == 1 || full_dest_multilocation.interior.len() == 2 {
        let (split_dest, beneficiary_junction) = full_dest_multilocation.split_last_interior();
        let beneficiary = MultiLocation::new(
            0,
            Junctions::X1(beneficiary_junction.ok_or(PublicError::InvalidMultiLocationLength)?),
        );
        Ok((split_dest, beneficiary))
    } else {
        Err(PublicError::InvalidMultiLocationLength)
    }
}

pub(crate) const fn get_dest_multilocation_template(
    src_chain_info: &ChainInfo,
    dest_chain_info: &ChainInfo,
) -> WalletMultiLocationTemplate {
    let address_tail_junction_template = match dest_chain_info.xcm_address_type {
        AddressType::Ethereum => {
            let zero_addr: [u8; 20] = [0; 20];
            Junction::AccountKey20 {
                network: NetworkId::Any,
                key: zero_addr,
            }
        }
        AddressType::SS58 => {
            let zero_addr: [u8; 32] = [0; 32];
            Junction::AccountId32 {
                network: NetworkId::Any,
                id: zero_addr,
            }
        }
    };
    let raw_multilocation = match (src_chain_info.chain_id, dest_chain_info.chain_id) {
        (
            UniversalChainId::SubstrateParachain(_, _),
            UniversalChainId::SubstrateParachain(_, dest_chain_id),
        ) => MultiLocation {
            parents: 1u8,
            interior: Junctions::X2(
                Junction::Parachain(dest_chain_id),
                address_tail_junction_template,
            ),
        },
        (
            UniversalChainId::SubstrateRelayChain(_),
            UniversalChainId::SubstrateParachain(_, dest_chain_id),
        ) => MultiLocation {
            parents: 0u8,
            interior: Junctions::X2(
                Junction::Parachain(dest_chain_id),
                address_tail_junction_template,
            ),
        },
        (UniversalChainId::SubstrateParachain(_, _), UniversalChainId::SubstrateRelayChain(_)) => {
            MultiLocation {
                parents: 1u8,
                interior: Junctions::X1(address_tail_junction_template),
            }
        }
        (UniversalChainId::SubstrateRelayChain(_), UniversalChainId::SubstrateRelayChain(_)) => {
            panic!("Hard fail. We should not be bridging across two relay chains")
        }
    };
    WalletMultiLocationTemplate {
        template: raw_multilocation,
    }
}

#[cfg(test)]
mod bridge_tests {
    use super::*;
    use hex_literal::hex;

    fn get_parachain_multilocation_address20(
        parents: u8,
        parachain: u32,
        dest: [u8; 20],
    ) -> MultiLocation {
        MultiLocation {
            parents: parents,
            interior: Junctions::X2(
                Junction::Parachain(parachain),
                Junction::AccountKey20 {
                    network: NetworkId::Any,
                    key: dest,
                },
            ),
        }
    }

    fn get_parachain_multilocation_address32(
        parents: u8,
        parachain: u32,
        dest: [u8; 32],
    ) -> MultiLocation {
        MultiLocation {
            parents: parents,
            interior: Junctions::X2(
                Junction::Parachain(parachain),
                Junction::AccountId32 {
                    network: NetworkId::Any,
                    id: dest,
                },
            ),
        }
    }

    fn get_relaychain_multilocation_address32(dest: [u8; 32]) -> MultiLocation {
        MultiLocation {
            parents: 1,
            interior: Junctions::X1(Junction::AccountId32 {
                network: NetworkId::Any,
                id: dest,
            }),
        }
    }

    #[test]
    fn test_moonbeam_to_astar_multilocation() {
        // A MultiLocation targeting an Astar address from Moonbeam:
        // e.g. https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fpublic-rpc.pinknode.io%2Fastar#/explorer/query/2493303
        // Note: Astar's polkadotXcm pallet has an identical interface to Polkadot relay's xcmPallet:
        // https://docs.astar.network/docs/xcm/building-with-xcm/client-applications/
        let multiloc_template = get_parachain_multilocation_address32(1, 2006, [0; 32]);
        let wallet_template = WalletMultiLocationTemplate {
            template: multiloc_template,
        };
        let addr = hex!("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
        let full_dest_multilocation = wallet_template
            .get_full_dest_multilocation_raw(addr)
            .unwrap();
        assert_eq!(
            full_dest_multilocation,
            get_parachain_multilocation_address32(1, 2006, addr)
        );
        let (dest, beneficiary) = wallet_template
            .get_split_dest_and_beneficiary_multilocation_raw(addr)
            .unwrap();
        assert_eq!(
            dest,
            MultiLocation {
                parents: 1,
                interior: Junctions::X1(Junction::Parachain(2006),),
            }
        );
        assert_eq!(
            beneficiary,
            MultiLocation {
                parents: 0,
                interior: Junctions::X1(Junction::AccountId32 {
                    network: NetworkId::Any,
                    id: addr
                },),
            }
        );
    }

    #[test]
    fn test_moonbeam_to_polkadot_multilocation() {
        // A MultiLocation targeting a Polkadot relay address from Moonbeam or Astar:
        // e.g. https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fwss.api.moonbeam.network#/explorer/query/2518311
        let multiloc_template = get_relaychain_multilocation_address32([0; 32]);
        let wallet_template = WalletMultiLocationTemplate {
            template: multiloc_template,
        };
        let addr = hex!("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
        let full_dest_multilocation = wallet_template
            .get_full_dest_multilocation_raw(addr)
            .unwrap();
        assert_eq!(
            full_dest_multilocation,
            get_relaychain_multilocation_address32(addr)
        );
        let (dest, beneficiary) = wallet_template
            .get_split_dest_and_beneficiary_multilocation_raw(addr)
            .unwrap();
        assert_eq!(
            dest,
            MultiLocation {
                parents: 1,
                interior: Junctions::Here
            }
        );
        assert_eq!(
            beneficiary,
            MultiLocation {
                parents: 0,
                interior: Junctions::X1(Junction::AccountId32 {
                    network: NetworkId::Any,
                    id: addr
                },),
            }
        );
    }

    #[test]
    fn test_polkadot_to_moonbeam_multilocation() {
        // A MultiLocation targeting a Moonbeam address from Polkadot relay
        let multiloc_template = get_parachain_multilocation_address20(0, 2004, [0; 20]);
        let wallet_template = WalletMultiLocationTemplate {
            template: multiloc_template,
        };
        let addr = hex!("000102030405060708090a0b0c0d0e0f10111213");
        let full_dest_multilocation = wallet_template
            .get_full_dest_multilocation_raw(addr)
            .unwrap();
        assert_eq!(
            full_dest_multilocation,
            get_parachain_multilocation_address20(0, 2004, addr)
        );
    }
}
