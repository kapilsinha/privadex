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

use core::fmt;
use ink_prelude::vec::Vec;
use serde::{de, Deserialize, Deserializer};

use privadex_common::{utils::general_utils::hex_string_to_vec, uuid::Uuid};

#[derive(Deserialize, Debug, PartialEq)]
#[allow(non_snake_case)]
pub(super) struct Empty {}

#[derive(Deserialize, Debug, PartialEq)]
#[allow(non_snake_case)]
pub(super) struct ItemWrapper<T> {
    pub Item: T,
}

#[derive(Deserialize, Debug, PartialEq)]
#[allow(non_snake_case)]
pub(super) struct AttributesWrapper<T> {
    pub Attributes: T,
}

#[derive(Deserialize, Debug, PartialEq)]
#[allow(non_snake_case)]
pub(super) struct NumWrapper {
    #[serde(deserialize_with = "quoted_str_to_u32")]
    pub N: u32,
}

#[derive(Deserialize, Debug, PartialEq)]
#[allow(non_snake_case)]
pub(super) struct MapWrapper<T> {
    pub M: T,
}

#[derive(Deserialize, Debug, PartialEq)]
#[allow(non_snake_case)]
pub(super) struct ExecPlanIdsWrapper {
    pub Plans: StringSet,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(bound(deserialize = "ink_prelude::vec::Vec<UuidContainer>: Deserialize<'de>"))]
#[allow(non_snake_case)]
pub(super) struct StringSet {
    pub SS: Vec<UuidContainer>,
}

#[derive(Deserialize, Debug, PartialEq)]
pub(super) struct UuidContainer(#[serde(deserialize_with = "str_to_uuid")] pub Uuid);

fn str_to_uuid<'de, D: Deserializer<'de>>(deserializer: D) -> core::result::Result<Uuid, D::Error> {
    let raw_string = <&str>::deserialize(deserializer)?;
    let hex_addr: [u8; 16] = hex_string_to_vec(raw_string)
        .map_err(|_| de::Error::custom("Invalid hex string for UUID"))?
        .try_into()
        .map_err(|_| de::Error::custom("UUID hex str length is incorrect"))?;
    Ok(Uuid::new(hex_addr))
}

#[derive(Deserialize, Debug, PartialEq)]
#[allow(non_snake_case)]
pub(super) struct PendingNonceBlockNextResponse {
    pub ExecStepPendingBlockAdded: MapWrapper<UnknownSingleKeyToNumWrapper>,
    pub ExecStepPendingNonce: MapWrapper<UnknownSingleKeyToNumWrapper>,
    pub NextNonce: NumWrapper,
}

#[derive(Deserialize, Debug, PartialEq)]
#[allow(non_snake_case)]
pub(super) struct PendingNonceBlockResponse {
    pub ExecStepPendingBlockAdded: MapWrapper<UnknownSingleKeyToNumWrapper>,
    pub ExecStepPendingNonce: MapWrapper<UnknownSingleKeyToNumWrapper>,
}

fn quoted_str_to_u32<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> core::result::Result<u32, D::Error> {
    let string = <&str>::deserialize(deserializer)?;
    let num: u32 = string
        .parse()
        .map_err(|_| de::Error::custom("String to u32 failed"))?;
    Ok(num)
}

#[derive(Debug, PartialEq)]
// Used to parse a json of the form "{\"unknown-key\":{\"N\":\"51\"}}"
// This requires custom deserialization because we cannot use HashMap in no_std
// and serde(rename) etc. macros must occur at compile time (but we don't know
// the value of the key until runtime)
pub(super) struct UnknownSingleKeyToNumWrapper {
    pub num: NumWrapper,
}

impl<'de> Deserialize<'de> for UnknownSingleKeyToNumWrapper {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UnknownKeyToNumWrapperVisitor;

        impl<'de> de::Visitor<'de> for UnknownKeyToNumWrapperVisitor {
            type Value = UnknownSingleKeyToNumWrapper;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct UnknownKeyToNumWrapper")
            }

            fn visit_map<V>(
                self,
                mut map: V,
            ) -> core::result::Result<UnknownSingleKeyToNumWrapper, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                // We throw away the name of the key and do not loop over keys because we assume there is
                // just one key
                let _ = map.next_key::<&str>()?;
                let val: NumWrapper = map.next_value()?;
                Ok(UnknownSingleKeyToNumWrapper { num: val })
            }
        }

        // FYI: FIELDS and the name in deserialize_struct are unnecessary for correct parsing.
        // But this form is standard
        const FIELDS: &'static [&'static str] = &["num"];
        deserializer.deserialize_struct(
            "UnknownKeyToNumWrapperVisitor",
            FIELDS,
            UnknownKeyToNumWrapperVisitor,
        )
    }
}

#[cfg(test)]
mod deserialize_helper_tests {
    use ink_prelude::vec;

    use super::*;

    #[test]
    fn test_execution_plan_deserialization() {
        let get_exec_plan_ids_response = "{\"Item\":{\"Plans\":{\"SS\":[\"0x01010101010101010101010101010101\",\"0x02020202020202020202020202020202\",\"0x04040404040404040404040404040404\"]}}}";
        let (decoded, _): (ItemWrapper<ExecPlanIdsWrapper>, usize) =
            serde_json_core::from_slice(get_exec_plan_ids_response.as_bytes())
                .expect("deserialize failed");
        assert_eq!(
            decoded,
            ItemWrapper {
                Item: ExecPlanIdsWrapper {
                    Plans: StringSet {
                        SS: vec![
                            UuidContainer(Uuid::new([1u8; 16])),
                            UuidContainer(Uuid::new([2u8; 16])),
                            UuidContainer(Uuid::new([4u8; 16])),
                        ]
                    }
                }
            }
        );
    }

    #[test]
    fn test_nonce_deserialization() {
        pink_extension_runtime::mock_ext::mock_all_ext();
        {
            let updated_block_nonce_next_response = "{\"Attributes\":{\"ExecStepPendingBlockAdded\":{\"M\":{\"execstep_0xcase2\":{\"N\":\"1001\"}}},\"ExecStepPendingNonce\":{\"M\":{\"execstep_0xcase2\":{\"N\":\"51\"}}},\"NextNonce\":{\"N\":\"52\"}}}";
            let (decoded, _): (AttributesWrapper<PendingNonceBlockNextResponse>, usize) =
                serde_json_core::from_slice(updated_block_nonce_next_response.as_bytes())
                    .expect("deserialize failed");
            assert_eq!(
                decoded,
                AttributesWrapper {
                    Attributes: PendingNonceBlockNextResponse {
                        ExecStepPendingBlockAdded: MapWrapper {
                            M: UnknownSingleKeyToNumWrapper {
                                num: NumWrapper { N: 1001 }
                            }
                        },
                        ExecStepPendingNonce: MapWrapper {
                            M: UnknownSingleKeyToNumWrapper {
                                num: NumWrapper { N: 51 }
                            }
                        },
                        NextNonce: NumWrapper { N: 52 }
                    }
                }
            );
        }
        {
            // https://serde.rs/no-std.html
            // NOTE: I need to specify ItemWrapper<PendingNonceBlockResponse> or ItemWrapper<Empty> depending on
            // whether the response is empty. It's absolutely ridiculous that serde_json_core requires you to write a custom
            // deserializer to combine these two into an enum. To avoid another custom deserializer, I will just
            // call both (and that will likely be as efficient as a custom deserializer)
            let get_block_nonce_response = "{\"Item\":{\"ExecStepPendingNonce\":{\"M\":{\"execstep_0xcase2\":{\"N\":\"51\"}}},\"ExecStepPendingBlockAdded\":{\"M\":{\"execstep_0xcase2\":{\"N\":\"1001\"}}}}}";

            // NOTE: You can also deserialize this as ItemWrapper<Empty>, which is incorrect behavior! Caution
            let (decoded, _): (ItemWrapper<PendingNonceBlockResponse>, usize) =
                serde_json_core::from_slice(get_block_nonce_response.as_bytes())
                    .expect("deserialize failed");
            assert_eq!(
                decoded,
                ItemWrapper {
                    Item: PendingNonceBlockResponse {
                        ExecStepPendingBlockAdded: MapWrapper {
                            M: UnknownSingleKeyToNumWrapper {
                                num: NumWrapper { N: 1001 }
                            }
                        },
                        ExecStepPendingNonce: MapWrapper {
                            M: UnknownSingleKeyToNumWrapper {
                                num: NumWrapper { N: 51 }
                            }
                        }
                    }
                }
            );

            let get_block_nonce_response_empty = "{\"Item\":{}}";
            let (decoded_empty, _): (ItemWrapper<Empty>, usize) =
                serde_json_core::from_slice(get_block_nonce_response_empty.as_bytes())
                    .expect("deserialize failed");
            assert_eq!(decoded_empty, ItemWrapper { Item: Empty {} });
        }
        {
            let reclaim_dropped_nonce_response= "{\"Attributes\":{\"ExecStepPendingNonce\":{\"M\":{\"execstep_0xcase4\":{\"N\":\"32\"}}},\"ExecStepPendingBlockAdded\":{\"M\":{\"execstep_0xcase4\":{\"N\":\"1001\"}}}}}";
            let (decoded, _): (AttributesWrapper<PendingNonceBlockResponse>, usize) =
                serde_json_core::from_slice(reclaim_dropped_nonce_response.as_bytes())
                    .expect("deserialize failed");
            assert_eq!(
                decoded,
                AttributesWrapper {
                    Attributes: PendingNonceBlockResponse {
                        ExecStepPendingBlockAdded: MapWrapper {
                            M: UnknownSingleKeyToNumWrapper {
                                num: NumWrapper { N: 1001 }
                            }
                        },
                        ExecStepPendingNonce: MapWrapper {
                            M: UnknownSingleKeyToNumWrapper {
                                num: NumWrapper { N: 32 }
                            }
                        }
                    }
                }
            );
        }
    }
}
