#![cfg_attr(not(feature = "std"), no_std)]

use pink_extension as pink;

pub mod chain_info;
pub mod eth_utils;
pub mod extrinsic_call_factory;
pub mod s3_api;
pub mod ss58_utils;
pub mod substrate_utils;


#[pink::contract(env=PinkEnvironment)]
mod phat_dex_aggregator {
    use super::*;
    use chain_info::{
        AddressType, ChainInfo, ChainToken, RelayChain, SignatureScheme, Ss58AddressFormat, xcm_prelude, universal_chain, UniversalChainId
    };
    use eth_utils::{Address as EthAddress, DEXRouterContract, H256, KeyPair, U128};
    use extrinsic_call_factory;
    use ss58_utils::Ss58Codec;
    use substrate_utils::{ExtrinsicSigConfig, slice_to_hex_string, SubstrateUtils};

    use pink::PinkEnvironment;
    use ink_env::debug_println;
    use ink_prelude::{
        string::String,
        vec::Vec,
        vec,
        format,
    };
    use ink_storage::traits::SpreadAllocate;

    use scale::{Decode, Encode};
    use sp_core::Pair;
    use sp_core::crypto::AccountId32;
    use sp_core_hashing;
    use sp_runtime::generic::Era;

    pub type Secret = [u8; 32];
    pub type Result<T> = core::result::Result<T, Error>;

    // const VERIFICATION_MSG: &'static str = "I verify that I submitted transaction {}";
    
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct PrivaDex {
        admin: AccountId,
        escrow_private_key: Option<Secret>,
        s3_access_key: Option<String>,
        s3_secret_key: Option<String>,
    }

    type BlockNum = u32;
    type EthTxnHash = H256;
    type NetworkName = String;
    type SrcNetworkName = NetworkName;
    type DestNetworkName = NetworkName;
    type SubstrateExtrinsicIndex = u32;
    type SubstrateExtrinsicHash = H256;

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ERC20XCSwapStateTransition {
        ReceivedFunds(NetworkName, EthTxnHash),
        XCTransferInitiated(SrcNetworkName, DestNetworkName, BlockNum /* last block num (used only for lookup later) */, SubstrateExtrinsicHash),
        XCTransferCompleted(SrcNetworkName, DestNetworkName, BlockNum, SubstrateExtrinsicIndex), // block + index containing extrinsic
        EthSwap(NetworkName, EthTxnHash),
        ERC20Transfer(NetworkName, EthTxnHash),
    }

    pub type ERC20XCSwapLedger = Vec<ERC20XCSwapStateTransition>;

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        AmountTooHigh,
        BadSignature,
        CallerIsNotTxnSubmitter,
        ExtrinsicNotFinalized,
        InvalidAddress,
        InvalidBody,
        InvalidKey,
        InvalidNetwork,
        NoPathFromSrcToDest,
        NoPermissions,
        NotANumber,
        NotImplemented,
        RequestFailed,
        TransactionFailed,
        TransactionNotFound,
        TransactionNotToEscrow,
        UninitializedEscrow,
        UnknownChain,
        UnsupportedChain,
        UnsupportedToken,
    }

    impl PrivaDex {
        #[ink(constructor)]
        pub fn new() -> Self {
            let admin = Self::env().caller();
            ink_lang::utils::initialize_contract(|this: &mut Self| {
                this.admin = admin;
                this.escrow_private_key = None;
                this.s3_access_key = None;
                this.s3_secret_key = None;
            })
        }

        #[ink(message)]
        pub fn init_secret_keys(
            &mut self,
            escrow_private_key: Secret, 
            s3_access_key: String,
            s3_secret_key: String,
        ) -> Result<()> {
            Self::guard(Self::env().caller() == self.admin, Error::NoPermissions)?;
            self.escrow_private_key = Some(escrow_private_key);
            self.s3_access_key = Some(s3_access_key);
            self.s3_secret_key = Some(s3_secret_key);
            Ok(())
        }

        #[ink(message)]
        pub fn get_admin(&self) -> AccountId {
            self.admin
        }

        // Self::env().caller() exists but we do not force the caller to have made the deposit.
        // This is also tricky with cross Substrate/Ethereum-like addresses
        #[ink(message)]
        pub fn swap_from_erc20_deposit(
            &self,
            _src_network_name: String,
            _erc20_deposit_txn: H256,
            _signed_verification: Vec<u8>,
            _dest_network_name: String,
            _dest_token: ChainToken,
            _dest_addr: Vec<u8>
        ) -> Result<ERC20XCSwapLedger> {
            // TODO: Call the intermediate helper functions
            Err(Error::NotImplemented)
        }

        #[ink(message)]
        pub fn initiate_xc_transfer_upon_received_funds(
            &self,
            src_network_name: String,
            erc20_deposit_txn: H256,
            signed_verification: Vec<u8>,
            dest_network_name: String,
        ) -> Result<(ERC20XCSwapStateTransition /* ::ReceivedFunds */, ERC20XCSwapStateTransition /* ::XCTransferInitiated */)> {
            // TODO: update S3 to ensure the same txn doesn't have swap called multiple times
            let src_chain_info = chain_info::get_chain_info(&src_network_name).ok_or(Error::InvalidNetwork)?;
            if src_chain_info.sig_scheme != SignatureScheme::Ethereum {
                return Err(Error::UnsupportedChain);
            }

            let signer_address = {
                let sv: [u8; 65] = signed_verification.try_into().map_err(|_| Error::BadSignature)?;
                let unprefixed_msg = self.get_verification_msg_to_sign(slice_to_hex_string(&erc20_deposit_txn.0));
                let prefixed_msg = src_chain_info.sig_scheme.prefix_msg(unprefixed_msg.as_bytes());
                let msg_hash = sp_core_hashing::keccak_256(&prefixed_msg);
                let mut pubkey = [0; 33];
                let _ = ink_env::ecdsa_recover(&sv, &msg_hash, &mut pubkey).map_err(|_| Error::BadSignature)?;
                Self::get_eth_address(&pubkey)?
            };

            // Note: This fails to parse native token transfers, only parses ERC20 transfers
            let transfer = eth_utils::parse_transfer_from_erc20_txn(&src_chain_info.rpc_url, erc20_deposit_txn).map_err(|_| Error::TransactionNotFound)?;
            let escrow_address = {
                let privkey = self.escrow_private_key.ok_or(Error::UninitializedEscrow)?;
                Self::get_eth_address_from_pair(&sp_core::ecdsa::Pair::from_seed(&privkey))?
            };
            if transfer.from != signer_address {
                debug_println!("transfer.from = {:?}, signer_address = {:?}", transfer.from, signer_address);
                return Err(Error::CallerIsNotTxnSubmitter);
            }
            if transfer.to != escrow_address {
                debug_println!("transfer.to = {:?}, escrow_address = {:?}", transfer.to, escrow_address);
                return Err(Error::TransactionNotToEscrow);
            }
            let transfer_amount = transfer.amount.low_u128();
            if transfer_amount > U128::MAX.low_u128() {
                debug_println!("transfer_amount = {:?}", transfer_amount);
                return Err(Error::AmountTooHigh);
            }
            // TODO: Add a lower bound

            let xc_transfer_initiated: ERC20XCSwapStateTransition /* ::XCTransferInitiated */ = self.initiate_xc_transfer(
                src_network_name.clone(),
                ChainToken::ERC20(transfer.token),
                transfer_amount,
                dest_network_name,
            )?;
            
            Ok((
                ERC20XCSwapStateTransition::ReceivedFunds(src_network_name, erc20_deposit_txn), 
                xc_transfer_initiated
            ))
        }

        #[ink(message)]
        pub fn eth_swap_upon_xc_transfer_completed(
            &self,
            network_name: String,
            src_token: ChainToken, // TODO: this should be parsed from the XC transfer eventually (but that isn't trivial cross-chain)
            amount_in_str: String, // should be u128 but the dumb UI does Javascript int parsing so it can't handle anything bigger than 2^53
            dest_token_eth_addr: EthAddress,
        ) -> Result<(ERC20XCSwapStateTransition /* ::XCTransferCompleted */, ERC20XCSwapStateTransition /* ::EthSwap */)> {
            let amount_in = u128::from_str_radix(&amount_in_str, 10).map_err(|_| Error::NotANumber)?;
            let key = {
                let privkey = self.escrow_private_key.ok_or(Error::UninitializedEscrow)?;
                KeyPair::from(privkey)
            };

            // TODO: This is obviously a placeholder. We need to read from S3 to populate the variables
            // Note this assumes cur_block > 10 or it will error out for overflow
            let xc_transfer_completed =
                self.find_xc_transfer_completed(network_name.clone(), network_name.clone(), self.cur_block() - 10, H256::zero())?;
            
            // This obviously is very over-simplified for the proof-of-concept. TODOs:
            // 0. EthAddress for src_token and dest_token needs to be passed in and matched with the AssetId definition
            // 1. Get actual amount_in from the dest_network (there is some fee taken on XC transfers)
            // 2. amount_out_min limit - currently we hard-code zero
            // 3. SOR (routing) logic - currently we assume a src/dest liquidity pool exists
            let chain_info = chain_info::get_chain_info(&network_name).ok_or(Error::UnknownChain)?;
            let escrow_addr = {
                let addr: [u8; 20] = self.get_escrow_account_pubkey(&chain_info)?.try_into().map_err(|_| Error::InvalidAddress)?;
                EthAddress{0: addr}
            };
            let dex_router = {
                let dex_router_addr = chain_info.eth_dex_router.ok_or(Error::UnsupportedChain)?;
                DEXRouterContract::new(&chain_info.rpc_url, dex_router_addr).map_err(|_| Error::InvalidNetwork)?
            };
            let deadline_millis = self.now_millis() + 600_000; // order is live for 10 minutes
            let txn_hash = match src_token {
                ChainToken::Native => {
                    let weth = dex_router.weth().map_err(|_| Error::RequestFailed)?;
                    dex_router.swap_exact_eth_for_tokens(
                        amount_in.into(),
                        0.into() /* amount_out_min */,
                        vec![weth, dest_token_eth_addr],
                        escrow_addr /* to */,
                        deadline_millis.into(),
                        &key,
                    ).map_err(|_| Error::TransactionFailed)?
                },
                _ => { return Err(Error::UnsupportedToken); }
            };
                
            Ok((
                xc_transfer_completed,
                ERC20XCSwapStateTransition::EthSwap(network_name, txn_hash)
            ))
        }

        pub fn find_xc_transfer_completed(
            &self,
            src_network_name: String,
            dest_network_name: String,
            start_block_number: BlockNum,
            _extrinsic_hash: SubstrateExtrinsicHash,
        ) -> Result<ERC20XCSwapStateTransition /* ::XCTransferCompleted */> {
            // TODO: This is dummy behavior. We just assume the extrinsic gets published in 4 blocks
            let src_chain_info = chain_info::get_chain_info(&src_network_name).ok_or(Error::UnknownChain)?;
            let _subutils = SubstrateUtils{ rpc_url: src_chain_info.rpc_url.clone() };
            let cur_block = self.cur_block();
            let (block_num, extrinsic_index) = {
                if cur_block >= start_block_number + 4 {
                    // TODO: Implement this
                    // let extrinsic_index = subutils.find_extrinsic(extrinsic_hash, cur_block).map_err(|_| Error::ExtrinsicNotFinalized)?;
                    // (cur_block, extrinsic_index)
                    (cur_block, 123)
                } else { return Err(Error::ExtrinsicNotFinalized); }
            };
            Ok(ERC20XCSwapStateTransition::XCTransferCompleted(src_network_name, dest_network_name, block_num, extrinsic_index))
        }

        #[ink(message)]
        pub fn erc20_transfer_upon_eth_swap_completed(
            &self,
            eth_swap_txn_hash: H256, // TODO: this should be read from S3. This is highly insecure rn
            network_name: String,
            dest_addr: EthAddress,
        ) -> Result<ERC20XCSwapStateTransition /* ::ERC20Transfer */> {
            let chain_info = chain_info::get_chain_info(&network_name).ok_or(Error::UnknownChain)?;
            let transfer = eth_utils::parse_transfer_from_dex_swap_txn(&chain_info.rpc_url, eth_swap_txn_hash).map_err(|_| Error::TransactionNotFound)?;
            let escrow_address = {
                let privkey = self.escrow_private_key.ok_or(Error::UninitializedEscrow)?;
                Self::get_eth_address_from_pair(&sp_core::ecdsa::Pair::from_seed(&privkey))?
            };

            if transfer.to != escrow_address {
                debug_println!("transfer.to = {:?}, escrow_address = {:?}", transfer.to, escrow_address);
                return Err(Error::TransactionNotToEscrow);
            }
            let transfer_amount = transfer.amount.low_u128();
            if transfer_amount > U128::MAX.low_u128() {
                debug_println!("transfer_amount = {:?}", transfer_amount);
                return Err(Error::AmountTooHigh);
            }
            self.erc20_transfer(network_name, transfer.token, transfer_amount, dest_addr)
        }

        #[ink(message)]
        pub fn get_verification_msg_to_sign(&self, txn_hash: String) -> String {
            format!("I verify that I submitted transaction {}", txn_hash)
            // format!(VERIFICATION_MSG, txn_hash)
        }
        
        #[ink(message)]
        pub fn get_escrow_account_address(&self, network_name: String) -> Result<String> {
            let privkey = self.escrow_private_key.ok_or(Error::UninitializedEscrow)?;
            let chain_info = chain_info::get_chain_info(&network_name).ok_or(Error::InvalidNetwork)?;
            match chain_info.sig_scheme.get_address_type() {
                AddressType::SS58 => {
                    let ss58_prefix = chain_info.ss58_prefix.ok_or(Error::InvalidNetwork)?;
                    let address = Self::get_ss58_address(
                        &sp_core::sr25519::Pair::from_seed(&privkey),
                        ss58_prefix
                    );
                    Ok(address)
                },
                AddressType::Ethereum => {
                    let address = Self::get_eth_address_from_pair(&sp_core::ecdsa::Pair::from_seed(&privkey))?;
                    Ok(slice_to_hex_string(&address.0))
                },
            }
        }

        fn get_escrow_account_pubkey(&self, chain_info: &ChainInfo) -> Result<Vec<u8>> {
            let privkey = self.escrow_private_key.ok_or(Error::UninitializedEscrow)?;
            let pubkey = match chain_info.sig_scheme.get_address_type() {
                AddressType::SS58 => sp_core::sr25519::Pair::from_seed(&privkey).public().0.to_vec(),
                AddressType::Ethereum => Self::get_eth_address_from_pair(&sp_core::ecdsa::Pair::from_seed(&privkey))?.0.to_vec(),
            };
            Ok(pubkey)
        }

        /*
         * Returns Ok()         if is_ok is true
         * Returns Error(error) if is_ok is false
         */
        fn guard(is_ok: bool, error: Error) -> Result<()> {
            match is_ok {
                true => Ok(()),
                false => Err(error),
            }
        }

        fn get_ss58_address(pair: &sp_core::sr25519::Pair, ss58_version: Ss58AddressFormat) -> String {
            AccountId32::from(pair.public().0).to_ss58check_with_version(ss58_version)
        }

        fn get_eth_address_from_pair(pair: &sp_core::ecdsa::Pair) -> Result<EthAddress> {
            Self::get_eth_address(&pair.public().0)
        }

        fn get_eth_address(pubkey: &[u8; 33]) -> Result<EthAddress> {
            let mut address = EthAddress::zero();
            if ink_env::ecdsa_to_eth_address(pubkey, &mut address.0).is_err() {
                return Err(Error::InvalidAddress);
            }
            Ok(address)
        }

        // Hacky way to populate a time in just the test environment because
        // env().block_timestamp() is 0 off chain and I can't figure out how to mock it
        #[cfg(test)]
        fn now_millis(&self) -> u64 {
            use std::time::SystemTime;
            SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis().try_into().unwrap()
        }

        #[cfg(not(test))]
        fn now_millis(&self) -> u64 {
            self.env().block_timestamp()
        }

        #[cfg(test)]
        fn cur_block(&self) -> u32 {
            11_000
        }

        #[cfg(not(test))]
        fn cur_block(&self) -> u32 {
            self.env().block_number()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use core::str::FromStr;
        use hex_literal::hex;
        use ink_env::debug_println;
        use ink_lang as ink;
        use substrate_utils::hex_string_to_vec;
        use openbrush::traits::mock::{Addressable, SharedCallStack};
        // use pink::chain_extension::{mock, HttpRequest, HttpResponse};
        // use pink::http_req;

        fn get_erc20_contract() -> eth_utils::ERC20Contract {
            let rpc_url = "https://moonbeam.public.blastapi.io";
            let token_address = EthAddress{0: hex!("FfFFfFff1FcaCBd218EDc0EbA20Fc2308C778080")}; // xcDOT
            eth_utils::ERC20Contract::new(&rpc_url, token_address).expect("Invalid ABI")
        }
        
        #[ink::test]
        fn erc20_name() {
            pink_extension_runtime::mock_ext::mock_all_ext();
            let name = get_erc20_contract().name().expect("Request failed");
            assert_eq!(name, "xcDOT");
        }

        #[ink::test]
        fn erc20xcswapledger_encode_works() {
            let ledger: ERC20XCSwapLedger =  vec![
                // Transfer VEN from 0x8097c3C354652CB1EEed3E5B65fBa2576470678A (Alice) to 0x05a81d8564a3eA298660e34e03E5Eff9a29d7a2A:
                // txn hash = 0x10cb2cafec121893abe065aa3c631777f7b21b08278b52aa3016cc092debee5e
                ERC20XCSwapStateTransition::ReceivedFunds(
                    "moonbase-alpha".to_string(),
                    EthTxnHash{0: hex!("10cb2cafec121893abe065aa3c631777f7b21b08278b52aa3016cc092debee5e")}
                ),
                // Extrinsic (sent from 0x05a81d8564a3eA298660e34e03E5Eff9a29d7a2A) =
                //    0x1e01010000010403000f0000c16ff2862301010200e10d030005a81d8564a3ea298660e34e03e5eff9a29d7a2a00ca9a3b00000000
                // (extrinsic hash = 0x8329dd8bd72016fc9488e45bb81ab8dfbace3b2a80ad97aa21e11e55c0ea15aa)
                ERC20XCSwapStateTransition::XCTransferInitiated(
                    "moonbase-alpha".to_string(),
                    "moonbase-beta".to_string(),
                    3_236_134u32,
                    SubstrateExtrinsicHash{0: hex!("8329dd8bd72016fc9488e45bb81ab8dfbace3b2a80ad97aa21e11e55c0ea15aa")}
                ),
                ERC20XCSwapStateTransition::XCTransferCompleted(
                    "moonbase-alpha".to_string(),
                    "moonbase-beta".to_string(),
                    3_236_136u32,
                    7u32
                ),
                // Fake transactions - TODO: maybe replace with actual ones later
                ERC20XCSwapStateTransition::EthSwap(
                    "moonbase-beta".to_string(),
                    EthTxnHash{0: hex!("0101010101010101010101010101010101010101010101010101010101010101")}
                ),
                ERC20XCSwapStateTransition::ERC20Transfer(
                    "moonbase-beta".to_string(),
                    EthTxnHash{0: hex!("0101010101010101010101010101010101010101010101010101010101010101")}
                )
            ];
            debug_println!("Ledger: {:?}", ledger);
            let encoded_ledger = ledger.encode();
            debug_println!("Encoded ledger: {:?}", encoded_ledger);
            let decoded_ledger = ERC20XCSwapLedger::decode(&mut &encoded_ledger[..]).expect("Decode should not have failed!");
            debug_println!("Decoded ledger: {:?}", decoded_ledger);
        }

        #[test]
        fn h256_encode_works() {
            pink_extension_runtime::mock_ext::mock_all_ext();
            let txn = H256{0: hex!("ff11ff11ff11ff11ff11ff11ff11ff11ff11ff11ff11ff11ff11ff11ff11ff11")}; // Alice
            debug_println!("test: {:?}", txn.encode());
        }

        #[test]
        fn chaintoken_encode_works() {
            pink_extension_runtime::mock_ext::mock_all_ext();
            debug_println!("Native token: {:?}", ChainToken::Native.encode());
            debug_println!("Relay token: {:?}", ChainToken::Relay.encode());
            debug_println!("ERC20 token: {:?}", ChainToken::ERC20(EthAddress{0: hex!("ffffffff1fcacbd218edc0eba20fc2308c778080")}).encode());
            debug_println!("GeneralAsset token: {:?}", ChainToken::GeneralAsset(999u128).encode());
        }

        fn get_phat_contract() -> Addressable<PrivaDex> {
            let kap_privkey = {
                let privkey_str = std::env::var("KAP_PRIVATE_KEY").expect("Env var KAP_PRIVATE_KEY is not set");
                H256::from_str(&privkey_str).expect("KAP_PRIVATE_KEY to_hex failed")
            };
            let kap_pubkey = ink_env::AccountId::try_from(sp_core::sr25519::Pair::from_seed(&kap_privkey.0).public().0).expect("Valid account");
            let stack = SharedCallStack::new(kap_pubkey.clone()); // accounts.alice
            stack.push(&kap_pubkey); // Sets the stack's caller before constructor is called (which sets admin)
            let contract = Addressable::create_native(1, PrivaDex::new(), stack.clone());

            let admin = contract.call().get_admin();
            debug_println!("Admin: {:?}", admin);
            let s3_access_key = std::env::var("S3_ACCESS_KEY").expect("Env var S3_ACCESS_KEY is not set");
            let s3_secret_key = std::env::var("S3_SECRET_KEY").expect("Env var S3_SECRET_KEY is not set");
            let _ = contract.call_mut().init_secret_keys(kap_privkey.0, s3_access_key, s3_secret_key).expect("Valid init");
            contract
        }

        #[test]
        fn get_verification_msg_works() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let contract = get_phat_contract();
            let msg = contract.call().get_verification_msg_to_sign("0x80ceab2d79fed91a042507bdf85142e846e4acdaaca4df4e86184b13a50c763c".to_string());
            debug_println!("Verification msg: {:?}", msg);
        }
        
        #[ink::test]
        fn end_to_end_step1() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let contract = get_phat_contract();

            // Note that this will actually send the XTransfer extrinsic.
            // Need to mock several HTTP calls to avoid this
            // I can't figure out how to access any field in request to conditionally
            // mock requests (I run into Borrow errors).
            // mock::mock_http_request(|request| {
                // HttpResponse::ok(br#"{"jsonrpc":"2.0","result":"0xabababababababababababababababababababababababababababababababab","id":1}"#.to_vec())
            // });
            let step1 = contract.call().initiate_xc_transfer_upon_received_funds(
                "moonbase-beta".to_string(), // src_dest_network_name
                H256{0: hex!("80ceab2d79fed91a042507bdf85142e846e4acdaaca4df4e86184b13a50c763c")}, // erc20_deposit_txn
                hex_string_to_vec("0x69618279b80dbb71798724716a422222017ddf52ac0034c84fc026664362c15923fe101efdf62fa30caf76da4d2b6af616888fd7fa18033e7cd20df0202db8fc00").unwrap(), // signed_verification
                "moonbase-alpha".to_string(), // dest_network_name
            );
            debug_println!("Step 1: {:?}", step1);
        }

        #[ink::test]
        fn end_to_end_step2() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let contract = get_phat_contract();    
            let step2 = contract.call().eth_swap_upon_xc_transfer_completed(
                "moonbase-alpha".to_string(), // network_name
                ChainToken::Native, // src_token = DEV
                "100000000000000".to_string(), // amount_in = 0.0001 DEV
                EthAddress{0: hex!("08B40414525687731C23F430CEBb424b332b3d35")} // dest_token_eth_addr = ERTH 
            );
            debug_println!("Step 2: {:?}", step2);
        }

        #[ink::test]
        fn end_to_end_step3() {
            pink_extension_runtime::mock_ext::mock_all_ext();

            let contract = get_phat_contract();
            let step3 = contract.call().erc20_transfer_upon_eth_swap_completed(
                H256{0: hex!("ba789a0d5c5971f4e9dd42a695e9b269f4d903cbcd71050a732ecf83bf702406")}, // eth_swap_txn_hash
                "moonbase-alpha".to_string(), // network_name
                EthAddress{0: hex!("8097c3C354652CB1EEed3E5B65fBa2576470678A")} // dest_addr = Alice
            );
            debug_println!("Step 3: {:?}", step3);
        }
    }
}
