// Copyright 2023 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::Header;

pub use pallet::*;
use sp_runtime::traits;

use melo_core_primitives::traits::{ExtendedHeader, HeaderCommitList};

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The block header.
		type ExtendedHeader: Parameter
			+ traits::Header<Number = Self::BlockNumber, Hash = Self::Hash>
			+ ExtendedHeader<Number = Self::BlockNumber, Hash = Self::Hash>;

		type CommitList: HeaderCommitList;
	}
}

impl<T: Config> Pallet<T> {
	/// Remove temporary "environment" entries in storage, compute the storage root and return the
	/// resulting header for this block.
	pub fn finalize() -> T::ExtendedHeader {
		let header = <frame_system::Pallet<T>>::finalize();
		let commit_list = T::CommitList::last();

		let mut ext_header = T::ExtendedHeader::new_ext(
			*header.number(),
			*header.extrinsics_root(),
			*header.state_root(),
			*header.parent_hash(),
			header.digest().clone(),
			Default::default(),
		);

		ext_header.set_commitments(&commit_list);

		// log::trace!(target: LOG_TARGET, "Header {:?}", header);
		ext_header
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate as frame_system_ext;
	use frame_support::{
		parameter_types,
		traits::{ConstU32, ConstU64},
	};
	use melo_core_primitives::{testing::CommitListTestWithData, Header as ExtendedHeader};
	use sp_core::H256;
	use sp_runtime::traits::{BlakeTwo256, IdentityLookup};

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
	}

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
	type Block = frame_system::mocking::MockBlock<Runtime>;

	pub type Header = ExtendedHeader<u64, BlakeTwo256>;

	// Mock runtime to test the module.
	frame_support::construct_runtime! {
		pub struct Runtime where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
			SystemExt: frame_system_ext::{Pallet},
		}
	}

	impl frame_system::Config for Runtime {
		type BaseCallFilter = frame_support::traits::Everything;
		type BlockWeights = ();
		type BlockLength = ();
		type DbWeight = ();
		type RuntimeOrigin = RuntimeOrigin;
		type Index = u64;
		type BlockNumber = u64;
		type RuntimeCall = RuntimeCall;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
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

	impl Config for Runtime {
		type ExtendedHeader = Header; // Mocked or a concrete type can be provided
		type CommitList = CommitListTestWithData; // Mocked or a concrete type can be provided
	}

	#[test]
	fn finalize_works() {
		let mut ext = sp_io::TestExternalities::new_empty();

		ext.execute_with(|| {
			let header = SystemExt::finalize();
			assert_eq!(header.extension, CommitListTestWithData::header_extension());
		});
	}
}
