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
#![cfg(test)]

use super::*;
use crate as melo_farmers_fortune;
use crate::mock::*;
use melo_proof_of_space::{mock::*, CellMetadata, PieceMetadata, PiecePosition, PreCell};

use frame_support::{assert_noop, assert_ok};

use melo_das_primitives::{KZGCommitment, Position};
use sp_core::H256;

#[test]
fn claim_reward_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(6);
		<frame_system::BlockHash<Runtime>>::insert(5, H256::from(BLOCK_HASH1));
		<frame_system::BlockHash<Runtime>>::insert(3, H256::from(BLOCK_HASH1));

		let segs = get_mock_row(&BLS_SCALAR11, &BLS_SCALAR12, 0, &PROOF_11, &PROOF_12, 16);
		let commit = KZGCommitment::try_from(COMMIT1).unwrap();

		let pre_cell = PreCell::new(PiecePosition::Row(0), segs[0].clone());
		let piece_metadata = PieceMetadata::new(3, PiecePosition::Row(0));

		let left_cell_metadata = CellMetadata::new(piece_metadata.clone(), 0);
		let right_cell_metadata = CellMetadata::new(piece_metadata, 1);

		let win_cell_left = Cell::new(left_cell_metadata, segs[0].clone());

		let win_cell_right = Cell::new(right_cell_metadata, segs[1].clone());

		assert_noop!(
			FarmersFortune::claim(
				RuntimeOrigin::signed(0),
				pre_cell.clone(),
				Box::new(win_cell_left.clone()),
				Box::new(win_cell_right.clone()),
			),
			melo_farmers_fortune::Error::<Runtime>::PreCommitNotFound
		);

		insert_mock_commitment(5, Position { x: 0, y: 0 }, commit);

		assert_noop!(
			FarmersFortune::claim(
				RuntimeOrigin::signed(0),
				pre_cell.clone(),
				Box::new(win_cell_left.clone()),
				Box::new(win_cell_right.clone()),
			),
			melo_farmers_fortune::Error::<Runtime>::WinCommitNotFound
		);

		insert_mock_commitment(3, Position { x: 0, y: 0 }, commit);

		assert_noop!(
			FarmersFortune::claim(
				RuntimeOrigin::signed(0),
				pre_cell.clone(),
				Box::new(win_cell_left.clone()),
				Box::new(win_cell_right.clone()),
			),
			melo_farmers_fortune::Error::<Runtime>::WinCommitNotFound
		);

		insert_mock_commitment(3, Position { x: 1, y: 0 }, commit);

		assert_ok!(FarmersFortune::claim(
			RuntimeOrigin::signed(0),
			pre_cell.clone(),
			Box::new(win_cell_left.clone()),
			Box::new(win_cell_right.clone()),
		));

		assert_noop!(
			FarmersFortune::claim(
				RuntimeOrigin::signed(0),
				pre_cell,
				Box::new(win_cell_left.clone()),
				Box::new(win_cell_right.clone()),
			),
			melo_farmers_fortune::Error::<Runtime>::AlreadyClaimed
		);
	});
}

#[test]
fn claim_reward_works_for_different_farmer_ids() {
	new_test_ext().execute_with(|| {
		System::set_block_number(6);
		<frame_system::BlockHash<Runtime>>::insert(5, H256::from(BLOCK_HASH1));
		<frame_system::BlockHash<Runtime>>::insert(3, H256::from(BLOCK_HASH1));

		let segs = get_mock_row(&BLS_SCALAR11, &BLS_SCALAR12, 0, &PROOF_11, &PROOF_12, 16);
		let commit = KZGCommitment::try_from(COMMIT1).unwrap();

		let pre_cell = PreCell::new(PiecePosition::Row(0), segs[0].clone());
		let piece_metadata = PieceMetadata::new(3, PiecePosition::Row(0));

		let left_cell_metadata = CellMetadata::new(piece_metadata.clone(), 0);
		let right_cell_metadata = CellMetadata::new(piece_metadata, 1);

		let win_cell_left = Cell::new(left_cell_metadata, segs[0].clone());

		let win_cell_right = Cell::new(right_cell_metadata, segs[1].clone());

		let wrong_commit = KZGCommitment::try_from(COMMIT2).unwrap();

		insert_mock_commitment(5, Position { x: 0, y: 0 }, wrong_commit);

		insert_mock_commitment(3, Position { x: 0, y: 0 }, commit);

		insert_mock_commitment(3, Position { x: 1, y: 0 }, commit);

		assert_noop!(
			FarmersFortune::claim(
				RuntimeOrigin::signed(0),
				pre_cell,
				Box::new(win_cell_left.clone()),
				Box::new(win_cell_right.clone()),
			),
			melo_farmers_fortune::Error::<Runtime>::InvalidSolution
		);
	});
}
