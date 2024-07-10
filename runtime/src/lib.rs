// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. This means the client is built with \
		 `SKIP_WASM_BUILD` flag and it is only usable for production chains. Please rebuild with \
		 the flag disabled.",
	)
}

pub mod config;
pub use config::core::*;
use config::{consensus, currency::*, gov, system, time::DAYS};

/// Runtime API definition for assets.
pub mod assets_api;
pub mod voter_bags;

use codec::{Decode, Encode};
use melo_auto_config::auto_config;
pub use node_primitives::{AccountId, Signature};
pub use node_primitives::{AccountIndex, Balance, BlockNumber, Hash, Moment, Nonce};
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};

// use sp_runtime::{
// 	generic, impl_opaque_keys,
// 	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
// 	traits::{BlakeTwo256, Block as BlockT, OpaqueKeys},
// };
use sp_runtime::{
	create_runtime_str, generic,
	generic::Era,
	impl_opaque_keys,
	traits::{self, BlakeTwo256, Block as BlockT, NumberFor, One, OpaqueKeys},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedU128, Perbill, Percent,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use frame_election_provider_support::{
	onchain, BalancingConfig, ElectionDataProvider, SequentialPhragmen, VoteWeight,
};
use pallet_election_provider_multi_phase::{GeometricDepositBase, SolutionAccuracyOf};
pub use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
#[cfg(feature = "std")]
pub use pallet_staking::StakerStatus;
use sp_consensus_grandpa::AuthorityId as GrandpaId;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	dynamic_params::{dynamic_pallet_params, dynamic_params},
	genesis_builder_helper::{build_config, create_default_config},
	pallet_prelude::Get,
	parameter_types,
	traits::{
		fungible::{
			Balanced, Credit, HoldConsideration, ItemOf, NativeFromLeft, NativeOrWithId, UnionOf,
		},
		tokens::{
			imbalance::ResolveAssetTo, nonfungibles_v2::Inspect, pay::PayAssetFromAccount,
			GetSalary, PayFromAccount,
		},
		AsEnsureOriginWithArg, ConstBool, ConstU128, ConstU16, ConstU32, ConstU64, ConstU8,
		Contains, Currency, EitherOfDiverse, EnsureOriginWithArg, EqualPrivilegeOnly, Everything,
		Imbalance, InsideBoth, InstanceFilter, KeyOwnerProofSystem, LinearStoragePrice,
		LockIdentifier, Nothing, OnUnbalanced, Randomness, SortedMembers, StorageInfo,
		WithdrawReasons,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight,
	},
	PalletId, StorageValue,
};
pub use frame_system::{
	limits::{BlockLength, BlockWeights},
	offchain::SendTransactionTypes,
	Call as SystemCall, EnsureRoot, EnsureRootWithSuccess, EnsureSigned, EnsureSignedBy,
};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, CurrencyAdapter, Multiplier};
use pallet_tx_pause::RuntimeCallNameOf;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
// pub use sp_runtime::{FixedU128, Perbill, Permill};

use melo_core_primitives::{
	block::ExtendedBlock as RuntimeExtendedBlock, Header as ExtendedHeader, SidecarMetadata,
};

pub use consensus::GENESIS_EPOCH_CONFIG;
use static_assertions::const_assert;
pub use system::BlockHashCount;

use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_std::vec;

/// Block header type as expected by this runtime.
pub type Header = ExtendedHeader<BlockNumber, BlakeTwo256>;

pub type Block = generic::Block<Header, UncheckedExtrinsic>;

pub type ExtendedBlock = RuntimeExtendedBlock<Header, UncheckedExtrinsic>;

pub type BlockId = generic::BlockId<Block>;

pub type SignedBlock = generic::SignedBlock<Block>;

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub im_online: ImOnline,
		pub authority_discovery: AuthorityDiscovery,
	}
}

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// Calls that can bypass the safe-mode pallet.
pub struct SafeModeWhitelistedCalls;
impl Contains<RuntimeCall> for SafeModeWhitelistedCalls {
	fn contains(call: &RuntimeCall) -> bool {
		matches!(call, RuntimeCall::System(_) | RuntimeCall::SafeMode(_) | RuntimeCall::TxPause(_))
	}
}

/// Calls that cannot be paused by the tx-pause pallet.
pub struct TxPauseWhitelistedCalls;
/// Whitelist `Balances::transfer_keep_alive`, all others are pauseable.
impl Contains<RuntimeCallNameOf<Runtime>> for TxPauseWhitelistedCalls {
	fn contains(full_name: &RuntimeCallNameOf<Runtime>) -> bool {
		matches!((full_name.0.as_slice(), full_name.1.as_slice()), (b"Balances", b"transfer_keep_alive"))
	}
}

impl pallet_safe_mode::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WhitelistedCalls = SafeModeWhitelistedCalls;
	type EnterDuration = EnterDuration;
	type EnterDepositAmount = EnterDepositAmount;
	type ExtendDuration = ExtendDuration;
	type ExtendDepositAmount = ExtendDepositAmount;
	type ForceEnterOrigin = EnsureRootWithSuccess<AccountId, ConstU32<9>>;
	type ForceExtendOrigin = EnsureRootWithSuccess<AccountId, ConstU32<11>>;
	type ForceExitOrigin = EnsureRoot<AccountId>;
	type ForceDepositOrigin = EnsureRoot<AccountId>;
	type ReleaseDelay = ReleaseDelay;
	type Notify = ();
	type WeightInfo = pallet_safe_mode::weights::SubstrateWeight<Runtime>;
}

impl pallet_tx_pause::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PauseOrigin = EnsureRoot<AccountId>;
	type UnpauseOrigin = EnsureRoot<AccountId>;
	type WhitelistedCalls = TxPauseWhitelistedCalls;
	type MaxNameLen = ConstU32<256>;
	type WeightInfo = pallet_tx_pause::weights::SubstrateWeight<Runtime>;
}

// Configure FRAME pallets to include in runtime.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type BaseCallFilter = InsideBoth<SafeMode, TxPause>;
	type BlockWeights = system::RuntimeBlockWeights;
	type BlockLength = system::RuntimeBlockLength;
	type DbWeight = RocksDbWeight;
	type Nonce = Nonce;
	type Hash = Hash;
	type AccountId = AccountId;
	type Lookup = Indices;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type Version = system::Version;
	type AccountData = pallet_balances::AccountData<Balance>;
	type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
	type SS58Prefix = system::SS58Prefix;
	type MaxConsumers = ConstU32<16>;
	type MultiBlockMigrator = MultiBlockMigrations;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

#[auto_config]
impl pallet_utility::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
}

use sp_runtime::traits::Convert;
pub struct BalanceToU256;
impl Convert<Balance, sp_core::U256> for BalanceToU256 {
	fn convert(balance: Balance) -> sp_core::U256 {
		sp_core::U256::from(balance)
	}
}
pub struct U256ToBalance;
impl Convert<sp_core::U256, Balance> for U256ToBalance {
	fn convert(n: sp_core::U256) -> Balance {
		n.try_into().unwrap_or(Balance::max_value())
	}
}

#[auto_config(skip_weight)]
impl pallet_nomination_pools::Config for Runtime {
	type WeightInfo = ();

	type Currency = Balances;
	type RewardCounter = FixedU128;
	type BalanceToU256 = BalanceToU256;
	type U256ToBalance = U256ToBalance;
	type Staking = Staking;
	type PostUnbondingPoolsWindow = consensus::PostUnbondPoolsWindow;
	type MaxMetadataLen = ConstU32<256>;
	type MaxUnbonding = ConstU32<8>;
	type PalletId = consensus::NominationPoolsPalletId;
	type MaxPointsToBalance = consensus::MaxPointsToBalance;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

#[auto_config]
impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type AccountStore = frame_system::Pallet<Runtime>;
	type MaxLocks = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;

	type MaxReserves = ();
	type DustRemoval = ();
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ();
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

#[auto_config(skip_weight)]
impl pallet_transaction_payment::Config for Runtime {
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;

	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
}

#[auto_config(skip_weight)]
impl pallet_sudo::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

#[auto_config(skip_weight)]
impl pallet_session::Config for Runtime {
	type WeightInfo = ();

	type ValidatorId = AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

frame_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = consensus::MaxElectingVoters,
	>(16)
);

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;

	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
	>;
	type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
	type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
	type Bounds = system::ElectionBoundsOnChain;
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = consensus::MinerMaxLength;
	type MaxWeight = consensus::MinerMaxWeight;
	type Solution = NposSolution16;
	type MaxVotesPerVoter =
    <<Self as pallet_election_provider_multi_phase::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
	type MaxWinners = consensus::MaxActiveValidators;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
        <Self as pallet_election_provider_multi_phase::Config>::WeightInfo
        as
        pallet_election_provider_multi_phase::WeightInfo
        >::submit_unsigned(v, t, a, d)
	}
}

/// The numbers configured here could always be more than the the maximum limits of staking pallet
/// to ensure election snapshot will not run out of memory. For now, we set them to smaller values
/// since the staking is bounded and the weight pipeline takes hours for this single pallet.
pub struct ElectionProviderBenchmarkConfig;

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = consensus::SignedPhase;
	type UnsignedPhase = consensus::UnsignedPhase;
	type OffchainRepeat = consensus::OffchainRepeat;
	type MinerTxPriority = consensus::MultiPhaseUnsignedPriority;
	type MinerConfig = Self;
	type SignedMaxSubmissions = ConstU32<10>;
	type SignedRewardBase = consensus::SignedRewardBase;
	type SignedDepositBase = GeometricDepositBase<
		Balance,
		system::SignedFixedDeposit,
		system::SignedDepositIncreaseFactor,
	>;
	type SignedDepositByte = consensus::SignedDepositByte;
	type SignedMaxRefunds = ConstU32<3>;
	type SignedMaxWeight = consensus::MinerMaxWeight;
	type DataProvider = Staking;
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
	type ForceOrigin = EnsureRootOrHalfCouncil;
	type MaxWinners = consensus::MaxActiveValidators;
	type ElectionBounds = system::ElectionBoundsMultiPhase;

	type BetterSignedThreshold = ();
	type SlashHandler = ();
	type RewardHandler = ();
	type SignedDepositWeight = ();

	type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Self>;
}

type EnsureRootOrHalfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

#[auto_config]
impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
	type AddOrigin = EnsureRootOrHalfCouncil;
	type RemoveOrigin = EnsureRootOrHalfCouncil;
	type SwapOrigin = EnsureRootOrHalfCouncil;
	type ResetOrigin = EnsureRootOrHalfCouncil;
	type PrimeOrigin = EnsureRootOrHalfCouncil;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
	type MaxMembers = gov::TechnicalMaxMembers;
}

type CouncilCollective = pallet_collective::Instance1;
#[auto_config]
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type Proposal = RuntimeCall;
	type MotionDuration = gov::CouncilMotionDuration;
	type MaxProposals = gov::CouncilMaxProposals;
	type MaxMembers = gov::CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = gov::MaxCollectivesProposalWeight;
	type RuntimeOrigin = RuntimeOrigin;
}

type TechnicalCollective = pallet_collective::Instance2;
#[auto_config]
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
	type MaxMembers = gov::TechnicalMaxMembers;
	type MaxProposals = gov::TechnicalMaxProposals;
	type MotionDuration = gov::TechnicalMotionDuration;
	type Proposal = RuntimeCall;
	type RuntimeOrigin = RuntimeOrigin;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = gov::MaxCollectivesProposalWeight;
}

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<1000>;
	type MaxValidators = ConstU32<1000>;
}

/// Upper limit on the number of NPOS nominations.
const MAX_QUOTA_NOMINATIONS: u32 = 16;

#[auto_config(include_currency)]
impl pallet_staking::Config for Runtime {
	type CurrencyBalance = Balance;
	/// A super-majority of the council can cancel the slash.
	type AdminOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
	>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type BondingDuration = consensus::BondingDuration;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	type ElectionProvider = ElectionProviderMultiPhase;
	type EraPayout = pallet_staking::ConvertCurve<consensus::RewardCurve>;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type HistoryDepth = consensus::HistoryDepth;
	type MaxUnlockingChunks = consensus::MaxUnlockingChunks;
	type NextNewSession = Session;
	type OffendingValidatorsThreshold = consensus::OffendingValidatorsThreshold;
	type RewardRemainder = Treasury;
	type SessionInterface = Self;
	// rewards are minted from the void
	type SessionsPerEra = consensus::SessionsPerEra;
	type Slash = Treasury;
	type SlashDeferDuration = consensus::SlashDeferDuration;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<MAX_QUOTA_NOMINATIONS>;
	type MaxExposurePageSize = ConstU32<256>;
	// This a placeholder, to be introduced in the next PR as an instance of bags-list
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type UnixTime = Timestamp;
	type VoterList = VoterList;
	type MaxControllersInDeprecationBatch = system::MaxControllersInDeprecationBatch;
	type EventListeners = NominationPools;

	// send the slashed funds to the treasury.
	type Reward = ();
}

#[auto_config(skip_weight)]
impl pallet_offences::Config for Runtime {
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
	config::system::RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 512;
}

#[auto_config]
impl pallet_scheduler::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
}

/// A source of random balance for NposSolver, which is meant to be run by the OCW election worker.
pub struct OffchainRandomBalancing;
impl Get<Option<BalancingConfig>> for OffchainRandomBalancing {
	fn get() -> Option<BalancingConfig> {
		use sp_runtime::traits::TrailingZeroInput;
		let iterations = match consensus::MINER_MAX_ITERATIONS {
			0 => 0,
			max => {
				let seed = sp_io::offchain::random_seed();
				let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
					.expect("input is padded with zeroes; qed")
					% max.saturating_add(1);
				random as usize
			},
		};

		let config = BalancingConfig { iterations, tolerance: 0 };
		Some(config)
	}
}

#[auto_config(include_currency)]
impl pallet_democracy::Config for Runtime {
	type EnactmentPeriod = gov::EnactmentPeriod;
	type LaunchPeriod = gov::LaunchPeriod;
	type VotingPeriod = gov::VotingPeriod;
	type VoteLockingPeriod = gov::EnactmentPeriod; // Same as EnactmentPeriod
	type MinimumDeposit = gov::MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
	type InstantOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
	type InstantAllowed = gov::InstantAllowed;
	type FastTrackVotingPeriod = gov::FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = gov::CooloffPeriod;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = gov::MaxVotes;
	type MaxProposals = gov::MaxProposals;
	type Preimages = Preimage;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
}

#[auto_config(include_currency)]
impl pallet_treasury::Config for Runtime {
	type PalletId = gov::TreasuryPalletId;
	type ApproveOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
	>;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
	>;
	type ProposalBond = gov::ProposalBond;
	type ProposalBondMinimum = gov::ProposalBondMinimum;
	type SpendPeriod = gov::SpendPeriod;
	type Burn = gov::Burn;
	type SpendFunds = Bounties;
	type MaxApprovals = gov::MaxApprovals;
	type SpendOrigin = frame_system::EnsureWithSuccess<
		frame_support::traits::EitherOf<
			EnsureRoot<AccountId>,
			pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
		>,
		AccountId,
		gov::MaxBalance,
	>;
	type AssetKind = u32;
	type Beneficiary = AccountId;
	type BeneficiaryLookup = Indices;
	type Paymaster = PayAssetFromAccount<Assets, gov::TreasuryAccount>;
	type BalanceConverter = AssetRate;
	type PayoutPeriod = gov::SpendPayoutPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();

	type OnSlash = ();
	type ProposalBondMaximum = ();
	type BurnDestination = ();
}

#[auto_config]
impl pallet_bounties::Config for Runtime {
	type BountyDepositBase = gov::BountyDepositBase;
	type BountyDepositPayoutDelay = gov::BountyDepositPayoutDelay;
	type BountyUpdatePeriod = gov::BountyUpdatePeriod;
	type CuratorDepositMultiplier = gov::CuratorDepositMultiplier;
	type CuratorDepositMin = gov::CuratorDepositMin;
	type CuratorDepositMax = gov::CuratorDepositMax;
	type BountyValueMinimum = gov::BountyValueMinimum;
	type DataDepositPerByte = gov::DataDepositPerByte;
	type MaximumReasonLength = gov::MaximumReasonLength;

	type ChildBountyManager = ();
}

impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = system::MaxAuthorities;
}

parameter_types! {
	pub const MaxSetIdSessionEntries: u32 = consensus::BondingDuration::get() * consensus::SessionsPerEra::get();
}

#[auto_config(skip_weight)]
impl pallet_grandpa::Config for Runtime {
	type WeightInfo = ();
	type MaxAuthorities = system::MaxAuthorities;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type EquivocationReportSystem = pallet_grandpa::EquivocationReportSystem<
		Self,
		Offences,
		Historical,
		consensus::ReportLongevity,
	>;
	type MaxNominators = system::MaxNominators;
}

parameter_types! {
	pub MbmServiceWeight: Weight = Perbill::from_percent(80) * system::RuntimeBlockWeights::get().max_block;
}

impl pallet_migrations::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Migrations = ();
	// Benchmarks need mocked migrations to guarantee that they succeed.
	#[cfg(feature = "runtime-benchmarks")]
	type Migrations = pallet_migrations::mock_helpers::MockedMigrations;
	type CursorMaxLen = ConstU32<65_536>;
	type IdentifierMaxLen = ConstU32<256>;
	type MigrationStatusHandler = ();
	type FailedMigrationHandler = frame_support::migrations::FreezeChainOnFailedMigration;
	type MaxServiceWeight = MbmServiceWeight;
	type WeightInfo = pallet_migrations::weights::SubstrateWeight<Runtime>;
}

impl pallet_babe::Config for Runtime {
	type WeightInfo = ();

	type EpochDuration = consensus::EpochDuration;
	type ExpectedBlockTime = consensus::ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type MaxAuthorities = system::MaxAuthorities;
	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
	type EquivocationReportSystem = pallet_babe::EquivocationReportSystem<
		Self,
		Offences,
		Historical,
		consensus::ReportLongevity,
	>;
	type MaxNominators = system::MaxNominators;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &voter_bags::THRESHOLDS;
}

type VoterBagsListInstance = pallet_bags_list::Instance1;
#[auto_config]
impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	/// The voter bags-list is loosely kept up to date, and the real source of truth for the score
	/// of each node is the staking pallet.
	type ScoreProvider = Staking;
	type BagThresholds = BagThresholds;
	type Score = VoteWeight;
}

#[auto_config]
impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type MaxKeys = consensus::MaxKeys;
	type MaxPeerInHeartbeats = consensus::MaxPeerInHeartbeats;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = consensus::ImOnlineUnsignedPriority;
	type ValidatorSet = Historical;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: <Signature as traits::Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(RuntimeCall, <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload)> {
		use sp_runtime::{traits::StaticLookup, SaturatedConversion as _};
		let tip = 0;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let extra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature, extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = DOLLARS;
	// One cent: $10,000 / MB
	pub const PreimageByteDeposit: Balance = CENTS;
}

#[auto_config(include_currency)]
impl pallet_preimage::Config for Runtime {
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		gov::PreimageHoldReason,
		LinearStoragePrice<
			dynamic_params::storage::BaseDeposit,
			dynamic_params::storage::ByteDeposit,
			Balance,
		>,
	>;
}

parameter_types! {
	pub const CandidacyBond: Balance = 10 * DOLLARS;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub const VotingBondBase: Balance = deposit(1, 64);
	// additional data per vote is 32 bytes (account id).
	pub const VotingBondFactor: Balance = deposit(0, 32);
	pub const TermDuration: BlockNumber = 7 * DAYS;
	pub const DesiredMembers: u32 = 13;
	pub const DesiredRunnersUp: u32 = 7;
	pub const MaxVotesPerVoter: u32 = 16;
	pub const MaxVoters: u32 = 512;
	pub const MaxCandidates: u32 = 64;
	pub const ElectionsPhragmenPalletId: LockIdentifier = *b"phrelect";
}

// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
const_assert!(DesiredMembers::get() <= gov::CouncilMaxMembers::get());

#[auto_config()]
impl pallet_elections_phragmen::Config for Runtime {
	type PalletId = ElectionsPhragmenPalletId;
	type Currency = Balances;
	type ChangeMembers = Council;
	// NOTE: this implies that council's genesis members cannot be set directly and must come from
	// this module.
	type InitializeMembers = Council;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	type CandidacyBond = CandidacyBond;
	type VotingBondBase = VotingBondBase;
	type VotingBondFactor = VotingBondFactor;
	type LoserCandidate = ();
	type KickedMember = ();
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
	type MaxVoters = MaxVoters;
	type MaxVotesPerVoter = MaxVotesPerVoter;
	type MaxCandidates = MaxCandidates;
}

parameter_types! {
	pub const AssetDeposit: Balance = 100 * DOLLARS;
	pub const ApprovalDeposit: Balance = DOLLARS;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10 * DOLLARS;
	pub const MetadataDepositPerByte: Balance = DOLLARS;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<DOLLARS>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
	type RemoveItemsLimit = ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const IndexDeposit: Balance = DOLLARS;
}

// #[auto_config()]
impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

impl frame_system_ext::Config for Runtime {
	type CommitList = MeloStore;
	type ExtendedBlock = ExtendedBlock;
}

#[auto_config(skip_weight)]
impl pallet_melo_store::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type MaxBlobNum = system::MaxBlobNumber;
	type MaxExtedLen = system::MaxExtedLen;
	type WeightInfo = ();
	type MeloUnsignedPriority = ();
	type MaxKeys = consensus::MaxKeys;
}

parameter_types! {
	pub const RewardAmount: Balance = 100 * DOLLARS;
	pub const MaxClaimantsPerBlock: u32 = 100;
}

// #[auto_config(skip_weight, include_currency)]
impl pallet_farmers_fortune::Config for Runtime {
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type CommitmentFromPosition = MeloStore;
	type RewardAmount = RewardAmount;
	type MaxClaimantsPerBlock = MaxClaimantsPerBlock;
}

impl pallet_asset_rate::Config for Runtime {
	type CreateOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = EnsureRoot<AccountId>;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type AssetKind = u32;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_asset_rate::weights::SubstrateWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

use sp_runtime::OpaqueExtrinsic;
/// Block type for the node
pub type NodeBlock = generic::Block<Header, OpaqueExtrinsic>;

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type SystemExt = frame_system_ext;

	#[runtime::pallet_index(2)]
	pub type Utility = pallet_utility;

	#[runtime::pallet_index(3)]
	pub type Babe = pallet_babe;

	#[runtime::pallet_index(4)]
	pub type Timestamp = pallet_timestamp;

	#[runtime::pallet_index(5)]
	pub type Authorship = pallet_authorship;

	#[runtime::pallet_index(6)]
	pub type Indices = pallet_indices;

	#[runtime::pallet_index(7)]
	pub type TransactionPayment = pallet_transaction_payment;

	#[runtime::pallet_index(8)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(9)]
	pub type ElectionProviderMultiPhase = pallet_election_provider_multi_phase;

	#[runtime::pallet_index(10)]
	pub type Staking = pallet_staking;

	#[runtime::pallet_index(11)]
	pub type Session = pallet_session;

	#[runtime::pallet_index(12)]
	pub type Democracy = pallet_democracy;

	#[runtime::pallet_index(13)]
	pub type Council = pallet_collective<Instance1>;

	#[runtime::pallet_index(14)]
	pub type TechnicalCommittee = pallet_collective<Instance2>;

	#[runtime::pallet_index(15)]
	pub type Elections = pallet_elections_phragmen;

	#[runtime::pallet_index(16)]
	pub type TechnicalMembership = pallet_membership<Instance1>;

	#[runtime::pallet_index(17)]
	pub type Grandpa = pallet_grandpa;

	#[runtime::pallet_index(18)]
	pub type Treasury = pallet_treasury;

	#[runtime::pallet_index(19)]
	pub type Sudo = pallet_sudo;

	#[runtime::pallet_index(20)]
	pub type ImOnline = pallet_im_online;

	#[runtime::pallet_index(21)]
	pub type AuthorityDiscovery = pallet_authority_discovery;

	#[runtime::pallet_index(22)]
	pub type Offences = pallet_offences;

	#[runtime::pallet_index(23)]
	pub type Historical = pallet_session_historical;

	#[runtime::pallet_index(24)]
	pub type Scheduler = pallet_scheduler;

	#[runtime::pallet_index(25)]
	pub type Preimage = pallet_preimage;

	#[runtime::pallet_index(26)]
	pub type Bounties = pallet_bounties;

	#[runtime::pallet_index(27)]
	pub type Assets = pallet_assets;

	#[runtime::pallet_index(28)]
	pub type VoterList = pallet_bags_list<Instance1>;

	#[runtime::pallet_index(29)]
	pub type NominationPools = pallet_nomination_pools;

	#[runtime::pallet_index(30)]
	pub type MeloStore = pallet_melo_store;

	#[runtime::pallet_index(31)]
	pub type FarmersFortune = pallet_farmers_fortune;

	#[runtime::pallet_index(32)]
	pub type SafeMode = pallet_safe_mode;

	#[runtime::pallet_index(33)]
	pub type TxPause = pallet_tx_pause;

	#[runtime::pallet_index(34)]
	pub type MultiBlockMigrations = pallet_migrations;

	#[runtime::pallet_index(35)]
	pub type AssetRate = pallet_asset_rate;

	#[runtime::pallet_index(36)]
	pub type Parameters = pallet_parameters;
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive_ext::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

/// Dynamic parameters that can be changed at runtime through the
/// `pallet_parameters::set_parameter`.
#[dynamic_params(RuntimeParameters, pallet_parameters::Parameters::<Runtime>)]
pub mod dynamic_params {
	use super::*;

	#[dynamic_pallet_params]
	#[codec(index = 0)]
	pub mod storage {
		/// Configures the base deposit of storing some data.
		#[codec(index = 0)]
		pub static BaseDeposit: Balance = DOLLARS;

		/// Configures the per-byte deposit of storing some data.
		#[codec(index = 1)]
		pub static ByteDeposit: Balance = CENTS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 1)]
	pub mod contracts {
		#[codec(index = 0)]
		pub static DepositPerItem: Balance = deposit(1, 0);

		#[codec(index = 1)]
		pub static DepositPerByte: Balance = deposit(0, 1);

		#[codec(index = 2)]
		pub static DefaultDepositLimit: Balance = deposit(1024, 1024 * 1024);
	}
}

pub struct DynamicParametersManagerOrigin;
impl EnsureOriginWithArg<RuntimeOrigin, RuntimeParametersKey> for DynamicParametersManagerOrigin {
	type Success = ();

	fn try_origin(
		origin: RuntimeOrigin,
		key: &RuntimeParametersKey,
	) -> Result<Self::Success, RuntimeOrigin> {
		match key {
			RuntimeParametersKey::Storage(_) => {
				frame_system::ensure_root(origin.clone()).map_err(|_| origin)?;
				Ok(())
			},
			RuntimeParametersKey::Contract(_) => {
				frame_system::ensure_root(origin.clone()).map_err(|_| origin)?;
				Ok(())
			},
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_key: &RuntimeParametersKey) -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

impl pallet_parameters::Config for Runtime {
	type RuntimeParameters = RuntimeParameters;
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = DynamicParametersManagerOrigin;
	type WeightInfo = ();
}

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]

		[pallet_babe, Babe]
		[pallet_indices, Indices]
		[pallet_timestamp, Timestamp]
		[pallet_balances, Balances]
		[pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[pallet_staking, Staking]
		[pallet_democracy, Democracy]
		[pallet_collective, Council]
		[pallet_collective, TechnicalCommittee]
		[pallet_grandpa, Grandpa]
		[pallet_treasury, Treasury]
		[pallet_im_online, ImOnline]
		[pallet_scheduler, Scheduler]
		[pallet_bounties, Bounties]
		[pallet_melo_store, MeloStore]
		[pallet_farmers_fortune, FarmersFortune]
		[pallet_elections_phragmen, Elections]
	);
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("melodot"),
	impl_name: create_runtime_str!("melodot"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

impl_runtime_apis! {
	impl melo_core_primitives::traits::Extractor<Block> for Runtime {
		fn extract(
			extrinsic: &Vec<u8>,
		) -> Option<Vec<SidecarMetadata>> {
			// Decode the unchecked extrinsic
			let extrinsic = UncheckedExtrinsic::decode(&mut &extrinsic[..]).ok()?;

			fn filter(call: RuntimeCall) -> Vec<SidecarMetadata> {
				match call {
					RuntimeCall::MeloStore(pallet_melo_store::Call::submit_data {
						params
					}) => vec![params],
					RuntimeCall::Utility(pallet_utility::Call::batch { calls })
					| RuntimeCall::Utility(pallet_utility::Call::batch_all { calls })
					| RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) => process_calls(calls),
					_ => vec![],
				}
			}

			fn process_calls(
				calls: Vec<RuntimeCall>,
			) -> Vec<SidecarMetadata> {
				calls.into_iter().flat_map(filter).collect()
			}

			Some(process_calls(vec![extrinsic.function]))
		}
	}

	impl melo_core_primitives::traits::AppDataApi<Block, RuntimeCall> for Runtime {

		fn get_blob_tx_param(function: &RuntimeCall) -> Option<SidecarMetadata> {
			match function {
				RuntimeCall::MeloStore(pallet_melo_store::Call::submit_data {
					params,
				}) => Some(params.clone()),
				_ => None,
			}
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl pallet_nomination_pools_runtime_api::NominationPoolsApi<Block, AccountId, Balance> for Runtime {
		fn pending_rewards(who: AccountId) -> Balance {
			NominationPools::api_pending_rewards(who).unwrap_or_default()
		}

		fn points_to_balance(pool_id: pallet_nomination_pools::PoolId, points: Balance) -> Balance {
			NominationPools::api_points_to_balance(pool_id, points)
		}

		fn balance_to_points(pool_id: pallet_nomination_pools::PoolId, new_funds: Balance) -> Balance {
			NominationPools::api_balance_to_points(pool_id, new_funds)
		}
	}

	impl pallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
		fn nominations_quota(balance: Balance) -> u32 {
			Staking::api_nominations_quota(balance)
		}

		fn eras_stakers_page_count(era: sp_staking::EraIndex, account: AccountId) -> sp_staking::Page {
			Staking::api_eras_stakers_page_count(era, account)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeConfiguration {
			let epoch_config = Babe::epoch_config().unwrap_or(GENESIS_EPOCH_CONFIG);
			sp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: consensus::EpochDuration::get() as u64,
				c: epoch_config.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: epoch_config.allowed_slots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			_authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl assets_api::AssetsApi<
		Block,
		AccountId,
		Balance,
		u32,
	> for Runtime
	{
		fn account_balances(account: AccountId) -> Vec<(u32, Balance)> {
			Assets::account_balances(account)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {

			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};
			use sp_storage::TrackedStorageKey;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn create_default_config() -> Vec<u8> {
			create_default_config::<RuntimeGenesisConfig>()
		}

		fn build_config(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_config::<RuntimeGenesisConfig>(config)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::traits::WhitelistedStorageKeys;
	use sp_core::hexdisplay::HexDisplay;
	use std::collections::HashSet;

	#[test]
	fn check_whitelist() {
		let whitelist: HashSet<String> = AllPalletsWithSystem::whitelisted_storage_keys()
			.iter()
			.map(|e| HexDisplay::from(&e.key).to_string())
			.collect();

		// Block Number
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac")
		);
		// Total Issuance
		assert!(
			whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80")
		);
		// Execution Phase
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a")
		);
		// Event Count
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850")
		);
		// System Events
		assert!(
			whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7")
		);
	}
}
