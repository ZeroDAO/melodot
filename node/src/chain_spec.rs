//! Substrate chain configurations.

use grandpa_primitives::AuthorityId as GrandpaId;
use melodot_runtime::{
	config::{consensus::MaxNominations, currency::*},
	wasm_binary_unwrap, AuthorityDiscoveryConfig, BabeConfig, BalancesConfig, Block, CouncilConfig,
	DemocracyConfig, ElectionsConfig, GrandpaConfig, ImOnlineConfig, IndicesConfig,
	NominationPoolsConfig, SessionConfig, SessionKeys, StakerStatus, StakingConfig, SudoConfig,
	SystemConfig, TechnicalCommitteeConfig,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

pub use melodot_runtime::GenesisConfig;
pub use node_primitives::{AccountId, Balance, Signature};
pub use helper::*;

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
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	initial_nominators: Vec<AccountId>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
	let default_endowed_accounts = vec![ALICE, BOB, CHARLIE, DAVE, EVE, FERDIE];
    let mut endowed_accounts = endowed_accounts.unwrap_or_else(|| endow_accounts(&default_endowed_accounts));

	// endow all authorities and nominators.
	initial_authorities
		.iter()
		.map(|x| &x.0)
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
		.map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
		.chain(initial_nominators.iter().map(|x| {
			use rand::{seq::SliceRandom, Rng};
			let limit = (MaxNominations::get() as usize).min(initial_authorities.len());
			let count = rng.gen::<usize>() % limit;
			let nominations = initial_authorities
				.as_slice()
				.choose_multiple(&mut rng, count)
				.into_iter()
				.map(|choice| choice.0.clone())
				.collect::<Vec<_>>();
			(x.clone(), x.clone(), STASH, StakerStatus::Nominator(nominations))
		}))
		.collect::<Vec<_>>();

	let num_endowed_accounts = endowed_accounts.len();

	GenesisConfig {
		system: SystemConfig { code: wasm_binary_unwrap().to_vec() },
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
		},
		indices: IndicesConfig { indices: vec![] },
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
		staking: StakingConfig {
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			..Default::default()
		},
		democracy: DemocracyConfig::default(),
		elections: ElectionsConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		},
		council: CouncilConfig::default(),
		technical_committee: TechnicalCommitteeConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		},
		sudo: SudoConfig { key: Some(root_key) },
		babe: BabeConfig {
			authorities: vec![],
			epoch_config: Some(melodot_runtime::GENESIS_EPOCH_CONFIG),
		},
		im_online: ImOnlineConfig { keys: vec![] },
		authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
		grandpa: GrandpaConfig { authorities: vec![] },
		technical_membership: Default::default(),
		treasury: Default::default(),
		transaction_payment: Default::default(),
		assets: pallet_assets::GenesisConfig {
			// This asset is used by the NIS pallet as counterpart currency.
			assets: vec![(9, sr25519_id(ALICE), true, 1)],
			..Default::default()
		},
		nomination_pools: NominationPoolsConfig {
			min_create_bond: 10 * DOLLARS,
			min_join_bond: 1 * DOLLARS,
			..Default::default()
		},
	}
}

fn development_config_genesis() -> GenesisConfig {
	testnet_genesis(
		authority_keys_from_seeds(&[ALICE]),
		vec![],
		sr25519_id(ALICE),
		None,
	)
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		development_config_genesis,
		vec![],
		None,
		None,
		None,
		None,
		Default::default(),
	)
}

fn local_testnet_genesis() -> GenesisConfig {
	testnet_genesis(
		authority_keys_from_seeds(&[ALICE, BOB]),
		vec![],
		sr25519_id(ALICE),
		None,
	)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		local_testnet_genesis,
		vec![],
		None,
		None,
		None,
		None,
		Default::default(),
	)
}

mod helper {
	use super::*;

	pub fn session_keys(
		grandpa: GrandpaId,
		babe: BabeId,
		im_online: ImOnlineId,
		authority_discovery: AuthorityDiscoveryId,
	) -> SessionKeys {
		SessionKeys { grandpa, babe, im_online, authority_discovery }
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
		accounts.iter()
			.flat_map(|&x| {
				vec![
					account_id_from_seed::<sr25519::Public>(x),
					account_id_from_seed::<sr25519::Public>(&format!("{}//stash", x))
				]
			})
			.collect()
	}
	
	/// Helper function to generate stash, controller and session key from seed
	pub fn authority_keys_from_seed(
		seed: &str,
	) -> (AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId) {
		(
			account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
			account_id_from_seed::<sr25519::Public>(seed),
			get_from_seed::<GrandpaId>(seed),
			get_from_seed::<BabeId>(seed),
			get_from_seed::<ImOnlineId>(seed),
			get_from_seed::<AuthorityDiscoveryId>(seed),
		)
	}
	
	pub fn authority_keys_from_seeds(
		seeds: &[&str],
	) -> Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> {
		seeds.iter().map(|x| authority_keys_from_seed(x)).collect()
	}
}
