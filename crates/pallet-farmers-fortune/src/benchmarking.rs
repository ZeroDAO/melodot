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

use super::*;
#[allow(unused_imports)]
use crate::Pallet as FarmersFortune;
use frame_benchmarking::v1::{benchmarks, impl_benchmark_test_suite};
use frame_system::{Pallet as System, RawOrigin};
use melo_das_primitives::KZGCommitment;
use melo_proof_of_space::{mock::*, CellMetadata, PieceMetadata, PiecePosition, PreCell};
use pallet_melo_store::Pallet as MeloStore;

benchmarks! {
	where_clause {
		where
			[u8; 32]: From<<T as frame_system::Config>::AccountId>,
			[u8; 32]: Into<<T as frame_system::Config>::AccountId>,
			[u8; 32]: From<<T as frame_system::Config>::Hash>,
			[u8; 32]: Into<<T as frame_system::Config>::Hash>,
			u32: From<BlockNumberFor<T>>,
			BalanceOf<T>: From<u128>,
			T: Config<CommitmentFromPosition = MeloStore<T>>,
			T: pallet_melo_store::Config,
	}

	claim  {
		let caller: T::AccountId = [0u8; 32].into();
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::from(100_000_000_0000u128));

		System::<T>::set_block_number(6u32.into());

		let header_hash: T::Hash = BLOCK_HASH1.into();
		let block_num5: BlockNumberFor<T> = 5u32.into();
		let block_num3: BlockNumberFor<T> = 3u32.into();

		<frame_system::BlockHash<T>>::insert(block_num5, header_hash);
		<frame_system::BlockHash<T>>::insert(block_num3, header_hash);

		let segs = get_mock_row(&BLS_SCALAR11, &BLS_SCALAR12, 0, &PROOF_11, &PROOF_12, 16);
		let commit = KZGCommitment::try_from(COMMIT1).unwrap();

		let pre_cell = PreCell::new(PiecePosition::Row(0), segs[0].clone());
		let piece_metadata = PieceMetadata::new(3u32.into(), PiecePosition::Row(0));

		let left_cell_metadata = CellMetadata::new(piece_metadata.clone(), 0);
		let right_cell_metadata = CellMetadata::new(piece_metadata, 1);

		let win_cell_left = Cell::new(left_cell_metadata, segs[0].clone());
		let win_cell_right = Cell::new(right_cell_metadata, segs[1].clone());

		let commit_vec = vec![commit.clone(),commit.clone(),commit.clone()];
		let _ = MeloStore::<T>::push_commitments_ext(block_num3, commit_vec.as_slice()).unwrap();
		let _ = MeloStore::<T>::push_commitments_ext(block_num5, commit_vec.as_slice()).unwrap();
	}: _(RawOrigin::Signed(caller),
		pre_cell.clone(),
		Box::new(win_cell_left.clone()),
		Box::new(win_cell_right.clone())
	)
	verify {}
}

impl_benchmark_test_suite!(FarmersFortune, crate::mock::new_test_ext(), crate::mock::Runtime);
