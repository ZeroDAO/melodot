//! Substrate chain configurations.

use grandpa_primitives::AuthorityId as GrandpaId;
use hex_literal::hex;
use melodot_runtime::{
	config::{consensus::MaxNominations, currency::*},
	BalancesConfig, Block, CouncilConfig, DemocracyConfig, ElectionsConfig, ImOnlineConfig,
	IndicesConfig, NominationPoolsConfig, SessionConfig, SessionKeys, StakerStatus, StakingConfig,
	SudoConfig, TechnicalCommitteeConfig, WASM_BINARY,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

pub use helper::*;
pub use melodot_runtime::RuntimeGenesisConfig;
pub use node_primitives::{AccountId, Balance, Signature};

type AccountPublic = <Signature as Verify>::Signer;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
	/// The light sync state extension used by the sync-state rpc.
	pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

/// Helper function to create GenesisConfig for testing
fn testnet_genesis(
	initial_authorities: Vec<AuthorityKeys>,
	initial_nominators: Vec<AccountId>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> serde_json::Value {
	let default_endowed_accounts = vec![ALICE, BOB, CHARLIE, DAVE, EVE, FERDIE];
	let mut endowed_accounts =
		endowed_accounts.unwrap_or_else(|| endow_accounts(&default_endowed_accounts));

	// endow all authorities and nominators.
	initial_authorities
		.iter()
		.map(|x| &x.controller)
		.chain(initial_nominators.iter())
		.for_each(|x| {
			if !endowed_accounts.contains(x) {
				endowed_accounts.push(x.clone())
			}
		});

	// stakers: all validators and nominators.
	let mut rng = rand::thread_rng();
	let stakers = initial_authorities
		.iter()
		.map(|x| (x.stash.clone(), x.controller.clone(), STASH, StakerStatus::Validator))
		.chain(initial_nominators.iter().map(|x| {
			use rand::{seq::SliceRandom, Rng};
			let limit = (MaxNominations::get() as usize).min(initial_authorities.len());
			let count = rng.gen::<usize>() % limit;
			let nominations = initial_authorities
				.as_slice()
				.choose_multiple(&mut rng, count)
				.map(|choice| choice.stash.clone())
				.collect::<Vec<_>>();
			(x.clone(), x.clone(), STASH, StakerStatus::Nominator(nominations))
		}))
		.collect::<Vec<_>>();

	let num_endowed_accounts = endowed_accounts.len();

	serde_json::json!( {
		"balances": BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
		},
		"indices": IndicesConfig { indices: vec![] },
		"session": SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (x.stash.clone(), x.stash.clone(), x.session_keys.clone()))
				.collect::<Vec<_>>(),
		},
		"staking": StakingConfig {
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities.iter().map(|x| x.stash.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			..Default::default()
		},
		"democracy": DemocracyConfig::default(),
		"elections": ElectionsConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		},
		"council": CouncilConfig::default(),
		"technical_committee": TechnicalCommitteeConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		},
		"sudo": SudoConfig { key: Some(root_key) },
		"babe":
		{
			"epochConfig": Some(melodot_runtime::GENESIS_EPOCH_CONFIG),
		},
		"im_online": ImOnlineConfig { keys: vec![] },
		"assets": {
			"assets": vec![(9, sr25519_id(ALICE), true, 1)]
		},
		"nomination_pools": NominationPoolsConfig {
			min_create_bond: 10 * DOLLARS,
			min_join_bond: DOLLARS,
			..Default::default()
		},
	})
}

fn development_config_genesis() -> serde_json::Value {
	testnet_genesis(authority_keys_from_seeds(&[ALICE]), vec![], sr25519_id(ALICE), None)
}

/// Development config (single validator Alice)
pub fn development_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Development")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_patch(development_config_genesis())
	.build())
}

fn local_testnet_genesis() -> serde_json::Value {
	testnet_genesis(
		authority_keys_from_seeds(&[ALICE, BOB, CHARLIE]),
		vec![],
		sr25519_id(ALICE),
		None,
	)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_patch(local_testnet_genesis())
	.build())
}

fn overtrue_testnet_genesis() -> serde_json::Value {
	let initial_authorities = vec![
		AuthorityKeys::from_accounts(
			// 5DhXG4yusZ98m7Tfm3doqvasUD2en6omxtLPXsW2hdS4rCcK
			hex!["4850a46ca0afedc00ad8239f903fe0348fc7f2dfb6af5100ef85f1d934871b1c"].into(),
			// 5DueQE9NG2GeRL4VQY5VpbyRweje5J6Wtiyre7vgvX6icMfL
			hex!["518f9fedc6b65cee2bfbe750e8fd92b24266e1b624c6359730e63081233ebcd3"]
				.unchecked_into(),
			None,
		),
		AuthorityKeys::from_accounts(
			// 5DUy3z85ad9s6qNZ16NjyeBRJzU1nfkGpUpxJoBKWzgcAr1W
			hex!["3ebd4644efe640497ffa8d6421bdf4f4d297b9e725a8bba08d61edad809bb871"].into(),
			// 5Hjyt9vrQ6Ljh1Th9tu1TMoR7U2vxa6FxSYgQ256jwQzYa1u
			hex!["fb21af619fe27e9963afe208f04d57d141624170a141a8fae2ada560941f44d5"]
				.unchecked_into(),
			None,
		),
		AuthorityKeys::from_accounts(
			// 5Gn5QQ5KaM98zgu4aDh1hZawpTg2wDfEDT7ope77MtYaBVwK
			hex!["d07e877f36ba61175dc769463fc131354338c382c95b8d6e9e36252ca05c6f79"].into(),
			// 5HZGUdqVinXiFQyTL6bFDuLqUhPyVnEE9iGonGHeGP8GdHDc
			hex!["f2f5d903a18828e3d4cb2dd1da7882900dbc9825eade91dcf61bcd1e25b0b817"]
				.unchecked_into(),
			None,
		),
	];

	testnet_genesis(
		initial_authorities,
		vec![],
		// 5F1NhF2fdsvBwFxAhQga9WjB344ECAJ8hDQJ99p1gWqpuzY1
		hex!["4850a46ca0afedc00ad8239f903fe0348fc7f2dfb6af5100ef85f1d934871b1c"].into(),
		// 5FCRAaNfM8wuYg6jQBHryeCXZSG68eLvHLdAYA6eWxukrbsG
		Some(vec![hex!["8a9679d0624a555c9c1088578b0c488b03c5150fe6c021f4968d338a6ebfb24b"].into()]),
	)
}

pub fn overtrue_testnet_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Overtrue Testnet")
	.with_id("overtrue_testnet")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_patch(overtrue_testnet_genesis())
	.build())
}

mod helper {
	use super::*;

	#[derive(Clone)]
	pub struct AuthorityKeys {
		pub controller: AccountId,
		pub stash: AccountId,
		pub session_keys: SessionKeys,
	}

	impl AuthorityKeys {
		/// Helper function to generate stash, controller and session key from seed
		pub fn from_seed(seed: &str) -> Self {
			let controller = account_id_from_seed::<sr25519::Public>(seed);
			let stash = account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed));
			let session_keys = SessionKeys {
				babe: get_from_seed::<BabeId>(seed),
				grandpa: get_from_seed::<GrandpaId>(seed),
				im_online: get_from_seed::<ImOnlineId>(seed),
				authority_discovery: get_from_seed::<AuthorityDiscoveryId>(seed),
			};

			Self { controller, stash, session_keys }
		}

		pub fn from_accounts(
			controller: AccountId,
			grandpa: GrandpaId,
			stash: Option<AccountId>,
		) -> Self {
			let raw: [u8; 32] = controller.clone().into();
			let stash = stash.unwrap_or_else(|| controller.clone());
			let session_keys = SessionKeys {
				babe: raw.unchecked_into(),
				grandpa,
				im_online: raw.unchecked_into(),
				authority_discovery: raw.unchecked_into(),
			};

			Self { controller, stash, session_keys }
		}
	}

	// Constants for default seeds and values
	pub const ALICE: &str = "Alice";
	pub const BOB: &str = "Bob";
	pub const CHARLIE: &str = "Charlie";
	pub const DAVE: &str = "Dave";
	pub const EVE: &str = "Eve";
	pub const FERDIE: &str = "Ferdie";

	pub const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
	pub const STASH: Balance = ENDOWMENT / 1000;

	/// Generates a public key from a seed.
	pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
		TPublic::Pair::from_string(&format!("//{}", seed), None)
			.expect("static values are valid; qed")
			.public()
	}

	/// Generates an account ID from a seed.
	pub fn account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
	where
		AccountPublic: From<<TPublic::Pair as Pair>::Public>,
	{
		AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
	}

	/// Generates a account key from a seed.
	pub fn sr25519_id(key: &str) -> AccountId {
		account_id_from_seed::<sr25519::Public>(key)
	}

	/// Endows a list of accounts with the default balance.
	pub fn endow_accounts(accounts: &[&str]) -> Vec<AccountId> {
		accounts
			.iter()
			.flat_map(|&x| {
				vec![
					account_id_from_seed::<sr25519::Public>(x),
					account_id_from_seed::<sr25519::Public>(&format!("{}//stash", x)),
				]
			})
			.collect()
	}

	pub fn authority_keys_from_seeds(seeds: &[&str]) -> Vec<AuthorityKeys> {
		seeds.iter().map(|x| AuthorityKeys::from_seed(x)).collect()
	}
}
