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

//! # Frame System Extension Module
//! 
//! This module provides an extension mechanism for the frame system.
//! It replaces the `finalize()` method in the original `frame_system::Pallet` module, allowing
//! for the generation of new types of block headers, introducing extended fields.
//! 
//! An alternative approach would be to directly modify the frame-system pallet, which would prevent 
//! modifications to the frame-executive module. However, this would make the frame-system cluttered 
//! and often require additional type conversions for blocks and block headers. Our modification to 
//! frame-executive is clear, involving simple type adjustments and the introduction of additional 
//! block headers and test tools. This ensures compatibility with the Substrate ecosystem.
//! 
//! ## Overview
//! 
//! The System Extension module introduces an extended block header that includes a commitment list, 
//! enhancing the capabilities of the traditional block header.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::Header;

pub use pallet::*;
use sp_runtime::traits;

use melo_core_primitives::traits::{ExtendedHeader, HeaderCommitList};

// Logger target for this module.
const LOG_TARGET: &str = "runtime::system_ext";

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configuration trait for this pallet.
	/// 
	/// This trait allows the definition of the extended block header and the commit list type.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The extended block header.
		type ExtendedHeader: Parameter
			+ traits::Header<Number = Self::BlockNumber, Hash = Self::Hash>
			+ ExtendedHeader<Number = Self::BlockNumber, Hash = Self::Hash>;

		/// The type of the commit list.
		type CommitList: HeaderCommitList;
	}
}

impl<T: Config> Pallet<T> {
	/// Finalizes the block creation process.
	/// 
	/// This function will:
	/// - Remove any temporary environmental storage entries.
	/// - Compute the storage root.
	/// - Return the resulting extended header for the current block.
	/// 
	/// # Returns
	/// 
	/// - `T::ExtendedHeader`: The extended block header with a commitment list.
	pub fn finalize() -> T::ExtendedHeader {
		// Retrieve the base header from the frame_system pallet.
		let header = <frame_system::Pallet<T>>::finalize();
		
		// Get the last commit list.
		let extension_data = T::CommitList::last();

		// Construct an extended header.
		let mut ext_header = T::ExtendedHeader::new_ext(
			*header.number(),
			*header.extrinsics_root(),
			*header.state_root(),
			*header.parent_hash(),
			header.digest().clone(),
			Default::default(),
		);

		// Set the commitments using the commit list.
		ext_header.set_extension(&extension_data);

		// Log the base header for debugging.
		log::trace!(target: LOG_TARGET, "Header {:?}", header);
		
		// Return the constructed extended header.
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
