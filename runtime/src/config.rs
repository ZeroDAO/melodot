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

//! A set of configuration types used to configure the FRAME pallets.
#![allow(clippy::identity_op)]
use frame_support::{
	dispatch::DispatchClass,
	parameter_types,
	traits::{ConstU16, ConstU32},
	weights::{constants::BlockExecutionWeight, Weight},
	PalletId,
};
use sp_runtime::{
	curve::PiecewiseLinear, transaction_validity::TransactionPriority, Perbill, Permill,
};
use static_assertions::const_assert;

use crate::{Balance, BlockLength, BlockNumber, BlockWeights, Moment, RuntimeVersion};

pub mod core {
	use super::*;
	use frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND;
	use sp_runtime::Perbill;

	/// This determines the average expected block time that we are targeting.
	/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
	/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
	/// up by `pallet_aura` to implement `fn slot_duration()`.
	///
	/// Change this to adjust the block time.
	pub const MILLISECS_PER_BLOCK: u64 = 6_000;

	pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

	/// Existential deposit.
	pub const EXISTENTIAL_DEPOSIT: u128 = 500;

	/// We allow for 2 seconds of compute with a 6 second average block time, with maximum proof size.
	pub const MAXIMUM_BLOCK_WEIGHT: Weight =
		Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

	/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
	/// This is used to limit the maximal weight of a single extrinsic.
	pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

	pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;

	// NOTE: Currently it is not possible to change the slot duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

	// 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

	pub const EPOCH_DURATION_IN_SLOTS: BlockNumber = 1 * time::HOURS;

	pub const MAX_BLOB_NUMBER: u32 = 100;
}

/// Money matters.
pub mod currency {
	use super::*;

	pub const MILLICENTS: Balance = 1_000_000_000;
	pub const CENTS: Balance = 1_000 * MILLICENTS;
	pub const DOLLARS: Balance = 100 * CENTS;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
	}
}

/// Time.
pub mod time {
	use super::*;

	// These time units are defined in number of blocks.
	pub const MINUTES: BlockNumber = 60 / (core::SECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	parameter_types! {
		// These time units are defined in number of blocks.
		pub storage Minutes: BlockNumber = 60 / (core::SECS_PER_BLOCK as BlockNumber);
		pub storage Hours: BlockNumber = MINUTES * 60;
		pub storage Days: BlockNumber = HOURS * 24;
	}
}

pub mod system {
	use super::*;
	use crate::VERSION;

	use frame_support::weights::constants::ExtrinsicBaseWeight;

	pub type MaxConsumers = ConstU32<16>;
	pub type SS58Prefix = ConstU16<42>;
	pub type MaxAuthorities = ConstU32<128>;

	parameter_types! {
		pub const BlockHashCount: BlockNumber = 2400;
		pub const Version: RuntimeVersion = VERSION;

		pub MaxBlobNumber: u32 = core::MAX_BLOB_NUMBER;
		pub MaxExtedLen: u32 = core::MAX_BLOB_NUMBER * 2;
		pub RuntimeBlockLength: BlockLength =
			BlockLength::max_with_normal_ratio(5 * 1024 * 1024, core::NORMAL_DISPATCH_RATIO);

		pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(core::NORMAL_DISPATCH_RATIO * core::MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(core::MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				core::MAXIMUM_BLOCK_WEIGHT - core::NORMAL_DISPATCH_RATIO * core::MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(core::AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	}
}

pub mod consensus {
	use super::*;

	use crate::{ElectionProviderBenchmarkConfig, NposSolution16};
	use sp_consensus_babe::{AllowedSlots, BabeEpochConfiguration};

	/// Maximum number of iterations for balancing that will be executed in the embedded OCW
	/// miner of election provider multi phase.
	pub const MINER_MAX_ITERATIONS: u32 = 10;

	impl pallet_election_provider_multi_phase::BenchmarkingConfig for ElectionProviderBenchmarkConfig {
		const VOTERS: [u32; 2] = [1000, 2000];
		const TARGETS: [u32; 2] = [500, 1000];
		const ACTIVE_VOTERS: [u32; 2] = [500, 800];
		const DESIRED_TARGETS: [u32; 2] = [200, 400];
		const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
		const MINER_MAXIMUM_VOTERS: u32 = 1000;
		const MAXIMUM_TARGETS: u32 = 300;
	}

	pallet_staking_reward_curve::build! {
		const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
			min_inflation: 0_025_000,
			max_inflation: 0_100_000,
			ideal_stake: 0_500_000,
			falloff: 0_050_000,
			max_piece_count: 40,
			test_precision: 0_005_000,
		);
	}

	/// The BABE epoch configuration at genesis.
	pub const GENESIS_EPOCH_CONFIG: BabeEpochConfiguration = BabeEpochConfiguration {
		c: core::PRIMARY_PROBABILITY,
		allowed_slots: AllowedSlots::PrimaryAndSecondaryVRFSlots,
	};

	pub type MaxUnlockingChunks = ConstU32<32>;

	parameter_types! {
		pub const EpochDuration: BlockNumber = core::EPOCH_DURATION_IN_SLOTS;
		pub const ExpectedBlockTime: Moment = core::MILLISECS_PER_BLOCK;

		// phase durations. 1/4 of the last session for each.
		pub const SignedPhase: u32 = core::EPOCH_DURATION_IN_SLOTS / 4;
		pub const UnsignedPhase: u32 = core::EPOCH_DURATION_IN_SLOTS / 4;

		pub ReportLongevity: BlockNumber =
			BondingDuration::get() * SessionsPerEra::get() * EpochDuration::get();

		// signed config
		pub const SignedRewardBase: Balance = 1 * currency::DOLLARS;
		pub const SignedDepositBase: Balance = 1 * currency::DOLLARS;
		pub const SignedDepositByte: Balance = 1 * currency::CENTS;

		pub BetterUnsignedThreshold: Perbill = Perbill::from_rational(1u32, 10_000);

		// worker configs
		pub const MultiPhaseUnsignedPriority: TransactionPriority = StakingUnsignedPriority::get() - 1u64;
		pub MinerMaxWeight: Weight = system::RuntimeBlockWeights::get()
			.get(DispatchClass::Normal)
			.max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
			.saturating_sub(BlockExecutionWeight::get());
		// Solution can occupy 90% of normal block size
		pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
			*system::RuntimeBlockLength::get()
			.max
			.get(DispatchClass::Normal);

		pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
		/// We prioritize im-online heartbeats over election solution submission.
		pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
		pub const MaxAuthorities: u32 = 100;
		pub const MaxKeys: u32 = 10_000;
		pub const MaxPeerInHeartbeats: u32 = 10_000;
		pub const MaxPeerDataEncodingSize: u32 = 1_000;

		pub const SessionsPerEra: sp_staking::SessionIndex = 6;
		pub const BondingDuration: sp_staking::EraIndex = 24 * 28;
		pub const SlashDeferDuration: sp_staking::EraIndex = 24 * 7; // 1/4 the bonding duration.
		pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
		pub const MaxNominatorRewardedPerValidator: u32 = 256;
		pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
		pub OffchainRepeat: BlockNumber = 5;
		pub HistoryDepth: u32 = 84;

		pub MaxNominations: u32 = <NposSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
		pub MaxElectingVoters: u32 = 40_000;
		pub MaxElectableTargets: u16 = 10_000;
		// OnChain values are lower.
		pub MaxOnChainElectingVoters: u32 = 5000;
		pub MaxOnChainElectableTargets: u16 = 1250;
		// The maximum winners that can be elected by the Election pallet which is equivalent to the
		// maximum active validators the staking pallet can have.
		pub MaxActiveValidators: u32 = 1000;

		pub const PostUnbondPoolsWindow: u32 = 4;
		pub const NominationPoolsPalletId: PalletId = PalletId(*b"py/nopls");
		pub const MaxPointsToBalance: u8 = 10;
	}
}

pub mod gov {
	use super::*;
	use currency::*;
	use sp_runtime::Percent;
	use time::*;

	parameter_types! {
		pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * system::RuntimeBlockWeights::get().max_block;

		// pallet_democracy
		pub LaunchPeriod: BlockNumber = 28 * 24 * 60 * Minutes::get();
		pub VotingPeriod: BlockNumber = 28 * 24 * 60 * Minutes::get();
		pub FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * Minutes::get();
		pub const InstantAllowed: bool = true;
		pub const MinimumDeposit: Balance = 100 * DOLLARS;
		pub EnactmentPeriod: BlockNumber = 30 * 24 * 60 * Minutes::get();
		pub CooloffPeriod: BlockNumber = 28 * 24 * 60 * Minutes::get();
		pub const MaxVotes: u32 = 100;
		pub const MaxProposals: u32 = 100;

		// pallet_treasury
		pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
		pub const BountyValueMinimum: Balance = 5 * DOLLARS;
		pub const BountyDepositBase: Balance = 1 * DOLLARS;
		pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
		pub const CuratorDepositMin: Balance = 1 * DOLLARS;
		pub const CuratorDepositMax: Balance = 100 * DOLLARS;
		pub const BountyDepositPayoutDelay: BlockNumber = 2 * MINUTES;
		pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;

		// pallet_bounties
		pub const ProposalBond: Permill = Permill::from_percent(5);
		pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
		pub const SpendPeriod: BlockNumber = 2 * MINUTES;
		pub const Burn: Permill = Permill::from_percent(1);
		pub const TipCountdown: BlockNumber = 1 * DAYS;
		pub const TipFindersFee: Percent = Percent::from_percent(20);
		pub const TipReportDepositBase: Balance = 1 * DOLLARS;
		pub const DataDepositPerByte: Balance = 1 * CENTS;
		pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
		pub const MaximumReasonLength: u32 = 300;
		pub const MaxApprovals: u32 = 100;
		pub const MaxBalance: Balance = Balance::max_value();

		// TechnicalCollective
		pub TechnicalMotionDuration: BlockNumber = 5 * Days::get();
		pub const TechnicalMaxProposals: u32 = 100;
		pub const TechnicalMaxMembers: u32 = 100;

		// CouncilCollective
		pub CouncilMotionDuration: BlockNumber = 5 * Days::get();
		pub const CouncilMaxProposals: u32 = 100;
		pub const CouncilMaxMembers: u32 = 100;
	}
}

const_assert!(
	core::NORMAL_DISPATCH_RATIO.deconstruct() >= core::AVERAGE_ON_INITIALIZE_RATIO.deconstruct()
);
