#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
// #[cfg(feature = "std")]
// include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod config;
use config::{consensus, core::*, currency::*, gov, system};

pub mod apis;
pub use apis::*;

pub mod voter_bags;

use codec::Decode;
use melo_auto_config::auto_config;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, One, OpaqueKeys, Verify,
	},
	MultiSignature,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use frame_election_provider_support::{
	onchain, BalancingConfig, ElectionDataProvider, SequentialPhragmen, VoteWeight,
};
use pallet_election_provider_multi_phase::SolutionAccuracyOf;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
#[cfg(feature = "std")]
pub use pallet_staking::StakerStatus;
use sp_consensus_grandpa::AuthorityId as GrandpaId;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime,
	dispatch::DispatchClass,
	pallet_prelude::Get,
	parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstU128, ConstU32, ConstU64, ConstU8, Currency, EitherOfDiverse,
		EqualPrivilegeOnly, Everything, Imbalance, InstanceFilter, KeyOwnerProofSystem,
		LockIdentifier, OnUnbalanced, Randomness, SortedMembers, StorageInfo, U128CurrencyToVote,
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
pub use frame_system::Call as SystemCall;
pub use frame_system::{
	limits::{BlockLength, BlockWeights},
	offchain::SendTransactionTypes,
	EnsureRoot, EnsureSigned, EnsureSignedBy,
};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, CurrencyAdapter, Multiplier};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{FixedU128, Perbill, Permill};

/// Import the template pallet.
pub use pallet_template;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Time type
pub type Moment = u64;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub grandpa: Grandpa,
			pub babe: Babe,
			pub im_online: ImOnline,
			pub authority_discovery: AuthorityDiscovery,
		}
	}
}

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = system::RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = system::RuntimeBlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = system::BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = system::Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = system::SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
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

#[auto_config(skip_weight, include_currency)]
impl pallet_nomination_pools::Config for Runtime {
	type WeightInfo = ();

	type RewardCounter = FixedU128;
	type BalanceToU256 = BalanceToU256;
	type U256ToBalance = U256ToBalance;
	type Staking = Staking;
	type PostUnbondingPoolsWindow = consensus::PostUnbondPoolsWindow;
	type MaxMetadataLen = ConstU32<256>;
	type MaxUnbonding = ConstU32<8>;
	type PalletId = consensus::NominationPoolsPalletId;
	type MaxPointsToBalance = consensus::MaxPointsToBalance;
}

#[auto_config]
impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type AccountStore = System;
	type MaxLocks = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;

	type MaxReserves = ();
	type DustRemoval = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type HoldIdentifier = ();
	type MaxHolds = ();
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
}

/// Configure the pallet-template in pallets/template.
impl pallet_template::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_template::weights::SubstrateWeight<Runtime>;
}

#[auto_config(skip_weight)]
impl pallet_session::Config for Runtime {
	type WeightInfo = ();

	type ValidatorId = AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
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
	type VotersBound = consensus::MaxOnChainElectingVoters;
	type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
	type TargetsBound = consensus::MaxOnChainElectableTargets;
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

#[auto_config(include_currency)]
impl pallet_election_provider_multi_phase::Config for Runtime {
	type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = consensus::SignedPhase;
	type UnsignedPhase = consensus::UnsignedPhase;
	type BetterUnsignedThreshold = consensus::BetterUnsignedThreshold;
	type OffchainRepeat = consensus::OffchainRepeat;
	type MinerTxPriority = consensus::MultiPhaseUnsignedPriority;
	type MinerConfig = Self;
	type SignedMaxSubmissions = ConstU32<10>;
	type SignedRewardBase = consensus::SignedRewardBase;
	type SignedDepositBase = consensus::SignedDepositBase;
	type SignedDepositByte = consensus::SignedDepositByte;
	type SignedMaxRefunds = ConstU32<3>;
	type SignedMaxWeight = consensus::MinerMaxWeight;
	type DataProvider = Staking;
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
	type ForceOrigin = EnsureRootOrHalfCouncil;
	type MaxElectableTargets = consensus::MaxElectableTargets;
	type MaxWinners = consensus::MaxActiveValidators;
	type MaxElectingVoters = consensus::MaxElectingVoters;

	type BetterSignedThreshold = ();
	type SlashHandler = ();
	type RewardHandler = ();
	type SignedDepositWeight = ();
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
	type CurrencyToVote = U128CurrencyToVote;
	type ElectionProvider = ElectionProviderMultiPhase;
	type EraPayout = pallet_staking::ConvertCurve<consensus::RewardCurve>;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type HistoryDepth = consensus::HistoryDepth;
	type MaxNominations = consensus::MaxNominations;
	type MaxNominatorRewardedPerValidator = consensus::MaxNominatorRewardedPerValidator;
	type MaxUnlockingChunks = consensus::MaxUnlockingChunks;
	type NextNewSession = Session;
	type OffendingValidatorsThreshold = consensus::OffendingValidatorsThreshold;
	type OnStakerSlash = NominationPools;
	type RewardRemainder = Treasury;
	type SessionInterface = Self;
	// rewards are minted from the void
	type SessionsPerEra = consensus::SessionsPerEra;
	type Slash = Treasury;
	type SlashDeferDuration = consensus::SlashDeferDuration;
	// This a placeholder, to be introduced in the next PR as an instance of bags-list
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type UnixTime = Timestamp;
	type VoterList = VoterList;

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
	type MaxPeerDataEncodingSize = consensus::MaxPeerDataEncodingSize;
	type MaxPeerInHeartbeats = consensus::MaxPeerInHeartbeats;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = consensus::ImOnlineUnsignedPriority;
	type ValidatorSet = Historical;
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
	pub const PreimageBaseDeposit: Balance = 1 * DOLLARS;
	// One cent: $10,000 / MB
	pub const PreimageByteDeposit: Balance = 1 * CENTS;
}

#[auto_config(include_currency)]
impl pallet_preimage::Config for Runtime {
	type ManagerOrigin = EnsureRoot<AccountId>;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime
	where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Timestamp: pallet_timestamp,
		Babe: pallet_babe,
		Grandpa: pallet_grandpa,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		Sudo: pallet_sudo,

		TemplateModule: pallet_template,

		Session: pallet_session,
		Staking: pallet_staking,
		VoterList: pallet_bags_list::<Instance1>,

		TechnicalCommittee: pallet_collective::<Instance2>,
		TechnicalMembership: pallet_membership::<Instance1>,
		Council: pallet_collective::<Instance1>,

		ElectionProviderMultiPhase: pallet_election_provider_multi_phase,
		Democracy: pallet_democracy,
		Treasury: pallet_treasury,
		ImOnline: pallet_im_online,
		AuthorityDiscovery: pallet_authority_discovery,
		Offences: pallet_offences,
		Historical: pallet_session_historical::{Pallet},
		NominationPools: pallet_nomination_pools,
		Scheduler: pallet_scheduler,
		Bounties: pallet_bounties,
		Preimage: pallet_preimage,
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
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
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_template, TemplateModule]
	);
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
