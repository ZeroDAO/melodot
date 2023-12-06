// Copyright 2023 ZeroDAO

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, sp_runtime::traits::Hash, traits::Currency};
use frame_system::pallet_prelude::*;
use melo_core_primitives::traits::CommitmentFromPosition;
use melo_proof_of_space::{Cell, Solution};
use sp_runtime::traits::BlakeTwo256;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for the extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// The method to get commitment from a position.
		type CommitmentFromPosition: CommitmentFromPosition<BlockNumber = Self::BlockNumber>;

		/// The currency trait.
		type Currency: Currency<Self::AccountId>;

		#[pallet::constant]
		type RewardAmount: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxClaimantsPerBlock: Get<u32>;
	}

	#[pallet::storage]
	pub type ClaimantsForBlock<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::BlockNumber,
		BoundedVec<T::AccountId, T::MaxClaimantsPerBlock>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a reward is claimed.
		RewardClaimed(T::AccountId, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error for report of a future block.
		InvalidSolution,
		/// Error when the pre-commitment is not found.
		PreCommitNotFound,
		/// Error when the win-commitment is not found.
		WinCommitNotFound,
		/// Error when the max claimants per block is reached.
		MaxClaimantsReached,
		/// Error when the user has already claimed.
		AlreadyClaimed,
		/// Error when the storage limit is reached.
		StorageLimitReached,
		///
		BlockNumberUnderflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Claim a reward.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::claim())]
		pub fn claim(
			origin: OriginFor<T>,
			pre_cell: Cell<BlockNumberFor<T>>,
			win_cell_left: Cell<BlockNumberFor<T>>,
			win_cell_right: Cell<BlockNumberFor<T>>,
			win_block_num: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let now = <frame_system::Pallet<T>>::block_number();

			let mut claimants = ClaimantsForBlock::<T>::get(now);
			ensure!(
				claimants.len() < T::MaxClaimantsPerBlock::get() as usize,
				Error::<T>::MaxClaimantsReached
			);
			ensure!(!claimants.contains(&who), Error::<T>::AlreadyClaimed);

			let pre_block_num = frame_support::sp_runtime::traits::CheckedSub::checked_sub(
				&now,
				&T::BlockNumber::from(1u32),
			)
			.ok_or(Error::<T>::BlockNumberUnderflow)?;

			let pre_block_hash = <frame_system::Pallet<T>>::block_hash(pre_block_num);
			let win_block_hash = <frame_system::Pallet<T>>::block_hash(win_block_num);

			let left_block_num = win_cell_left.metadata.block_number();
			let right_block_num = win_cell_right.metadata.block_number();

			// Get commitments from positions
			let pre_commit = T::CommitmentFromPosition::get_commitment(
				pre_block_num,
				&pre_cell.metadata.seg_position(),
			)
			.ok_or(Error::<T>::PreCommitNotFound)?;

			let left_commit = T::CommitmentFromPosition::get_commitment(
				left_block_num,
				&win_cell_left.metadata.seg_position(),
			)
			.ok_or(Error::<T>::WinCommitNotFound)?;

			let right_commit = T::CommitmentFromPosition::get_commitment(
				right_block_num,
				&win_cell_right.metadata.seg_position(),
			)
			.ok_or(Error::<T>::WinCommitNotFound)?;

			let farmer_id = BlakeTwo256::hash_of(&who).into();

			let solution = Solution::<T::Hashing, BlockNumberFor<T>>::new(
				&pre_block_hash,
				&farmer_id,
				&pre_cell,
				&win_cell_left,
				&win_cell_right,
			);

			ensure!(
				solution.verify(
					&pre_commit,
					&left_commit,
					&right_commit,
					&win_block_hash,
					&win_block_hash,
					1,
				),
				Error::<T>::InvalidSolution
			);

			claimants.try_push(who.clone()).map_err(|_| Error::<T>::StorageLimitReached)?;
			ClaimantsForBlock::<T>::insert(now, claimants);

			let reward = T::RewardAmount::get();
			T::Currency::deposit_creating(&who, reward);

			Self::deposit_event(Event::RewardClaimed(who, reward));

			Ok(().into())
		}
	}
}
