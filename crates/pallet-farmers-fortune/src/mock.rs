// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Test utilities

#![cfg(test)]
#![allow(non_camel_case_types)]

use frame_support::{
	derive_impl, parameter_types,
	traits::{ConstU32, ConstU64},
};
use lazy_static::lazy_static;
use melo_core_primitives::traits::CommitmentFromPosition;
use melo_das_primitives::{KZGCommitment, Position};
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use sp_std::sync::RwLock;
use std::collections::HashMap;

use super::*;
use crate as melo_farmers_fortune;
use crate::Config;

pub(crate) type Balance = u64;

type Block = frame_system::mocking::MockBlock<Runtime>;

frame_support::construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		Balances: pallet_balances,
		FarmersFortune: melo_farmers_fortune::{Pallet, Call, Storage, Event<T>},
	}
);

lazy_static! {
	static ref MOCK_COMMITMENTS: RwLock<HashMap<(u64, Position), KZGCommitment>> =
		RwLock::new(HashMap::new());
}

pub struct MockCommitmentFromPosition;

impl CommitmentFromPosition for MockCommitmentFromPosition {
	type BlockNumber = u64;

	fn commitments(block_number: Self::BlockNumber, position: &Position) -> Option<KZGCommitment> {
		MOCK_COMMITMENTS
			.read()
			.expect("RwLock is poisoned")
			.get(&(block_number, position.clone()))
			.cloned()
	}
}

pub fn insert_mock_commitment(block_number: u64, position: Position, commitment: KZGCommitment) {
	MOCK_COMMITMENTS
		.write()
		.expect("RwLock is poisoned")
		.insert((block_number, position), commitment);
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const RewardAmount: Balance = 1000;
}

impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type CommitmentFromPosition = MockCommitmentFromPosition;
	type Currency = Balances;
	type RewardAmount = RewardAmount;
	type MaxClaimantsPerBlock = ConstU32<2>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	t.into()
}
