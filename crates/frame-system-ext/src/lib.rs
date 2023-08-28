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
