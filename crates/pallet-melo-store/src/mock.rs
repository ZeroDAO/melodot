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
	weights::Weight,
};
use pallet_im_online as imonline;
use pallet_session::historical as pallet_session_historical;
use sp_core::H256;
use sp_runtime::{
	testing::{TestXt, UintAuthorityId},
	traits::{BlakeTwo256, ConvertInto, IdentityLookup},
	BuildStorage, Permill,
};
use sp_staking::{
	offence::{OffenceError, ReportOffence},
	SessionIndex,
};

use crate as pallet_melo_store;
use crate::Config;

type Block = frame_system::mocking::MockBlock<Runtime>;

frame_support::construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		Session: pallet_session,
		MeloStore: pallet_melo_store::{Pallet, Call, Storage, Event<T>},
		Historical: pallet_session_historical,
	}
);

parameter_types! {
	pub static Validators: Option<Vec<u64>> = Some(vec![
		1,
		2,
		3,
	]);
}

pub struct TestSessionManager;
impl pallet_session::SessionManager<u64> for TestSessionManager {
	fn new_session(_new_index: SessionIndex) -> Option<Vec<u64>> {
		Validators::mutate(|l| l.take())
	}
	fn end_session(_: SessionIndex) {}
	fn start_session(_: SessionIndex) {}
}

impl pallet_session::historical::SessionManager<u64, u64> for TestSessionManager {
	fn new_session(_new_index: SessionIndex) -> Option<Vec<(u64, u64)>> {
		Validators::mutate(|l| {
			l.take().map(|validators| validators.iter().map(|v| (*v, *v)).collect())
		})
	}
	fn end_session(_: SessionIndex) {}
	fn start_session(_: SessionIndex) {}
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<RuntimeCall, ()>;
type IdentificationTuple = (u64, u64);
type Offence = imonline::UnresponsivenessOffence<IdentificationTuple>;

parameter_types! {
	pub static Offences: Vec<(Vec<u64>, Offence)> = vec![];
}

/// A mock offence report handler.
pub struct OffenceHandler;
impl ReportOffence<u64, IdentificationTuple, Offence> for OffenceHandler {
	fn report_offence(reporters: Vec<u64>, offence: Offence) -> Result<(), OffenceError> {
		Offences::mutate(|l| l.push((reporters, offence)));
		Ok(())
	}

	fn is_known_offence(_offenders: &[IdentificationTuple], _time_slot: &SessionIndex) -> bool {
		false
	}
}

parameter_types! {
	pub static Ongoing: bool = false;
}

pub struct MockedMigrator;
impl frame_support::migrations::MultiStepMigrator for MockedMigrator {
	fn ongoing() -> bool {
		Ongoing::get()
	}

	fn step() -> Weight {
		Weight::zero()
	}
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
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const Period: u64 = 1;
	pub const Offset: u64 = 0;
}

impl pallet_session::Config for Runtime {
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager =
		pallet_session::historical::NoteHistoricalRoot<Runtime, TestSessionManager>;
	type SessionHandler = (MeloStore,);
	type ValidatorId = u64;
	type ValidatorIdOf = ConvertInto;
	type Keys = UintAuthorityId;
	type RuntimeEvent = RuntimeEvent;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type WeightInfo = ();
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = u64;
	type FullIdentificationOf = ConvertInto;
}

parameter_types! {
	pub static MockCurrentSessionProgress: Option<Option<Permill>> = None;
}

parameter_types! {
	pub static MockAverageSessionLength: Option<u64> = None;
}

pub struct TestNextSessionRotation;

impl frame_support::traits::EstimateNextSessionRotation<u64> for TestNextSessionRotation {
	fn average_session_length() -> u64 {
		// take the mock result if any and return it
		let mock = MockAverageSessionLength::mutate(|p| p.take());

		mock.unwrap_or(pallet_session::PeriodicSessions::<Period, Offset>::average_session_length())
	}

	fn estimate_current_session_progress(now: u64) -> (Option<Permill>, Weight) {
		let (estimate, weight) =
			pallet_session::PeriodicSessions::<Period, Offset>::estimate_current_session_progress(
				now,
			);

		// take the mock result if any and return it
		let mock = MockCurrentSessionProgress::mutate(|p| p.take());

		(mock.unwrap_or(estimate), weight)
	}

	fn estimate_next_session_rotation(now: u64) -> (Option<u64>, Weight) {
		pallet_session::PeriodicSessions::<Period, Offset>::estimate_next_session_rotation(now)
	}
}

pub const MAX_BLOB_NUM: u32 = 10;

parameter_types! {
	pub const MaxBlobNum: u32 = MAX_BLOB_NUM;
	pub const MaxExtedLen: u32 = MAX_BLOB_NUM * 2;
}

impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type AuthorityId = UintAuthorityId;
	type MaxKeys = ConstU32<10_000>;
	type MaxBlobNum = MaxBlobNum;
	type MaxExtedLen = MaxExtedLen;
	type MeloUnsignedPriority = ConstU64<{ 1 << 20 }>;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	let mut result: sp_io::TestExternalities = t.into();
	// Set the default keys, otherwise session will discard the validator.
	result.execute_with(|| {
		for i in 1..=6 {
			System::inc_providers(&i);
			assert_eq!(Session::set_keys(RuntimeOrigin::signed(i), (i - 1).into(), vec![]), Ok(()));
		}
	});
	result
}

pub fn advance_session() {
	let now = System::block_number().max(1);
	System::set_block_number(now + 1);
	Session::rotate_session();
	let keys = Session::validators().into_iter().map(UintAuthorityId).collect();
	MeloStore::set_keys(keys);
	assert_eq!(Session::current_index(), (now / Period::get()) as u32);
}
