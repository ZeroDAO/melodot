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

use crate::{
	utils, ChaCha8, Decode, Encode, FarmerId, HashT, Key, KeyIvInit, Nonce, Position, StreamCipher,
};
use core::marker::PhantomData;

/// `XValueManager` manages the generation and iteration of X values for a given block number,
/// farmer ID, cell index, and position. This struct is used to generate X values for a segment.
///
/// Type parameters:
/// * `H`: A hash type implementing `HashT`.
/// * `BlockNumber`: The type representing the block number.
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq, Encode, Decode)]
pub struct XValueManager<H: HashT, BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash,
{
	// The block number of block that cell is in.
	block_num: BlockNumber,
	// The farmer ID.
	farmer_id: FarmerId,
	// The cell index of the segment.
	cell_index: u32,
	// The position of the cell in the segment.
	position: Position,
	phantom: PhantomData<H>,
}

impl<H: HashT, BlockNumber> XValueManager<H, BlockNumber>
where
	BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
{
	/// Creates a new `XValueManager` instance.
	///
	/// Parameters:
	/// * `farmer_id`: The farmer's ID.
	/// * `block_num`: The block number.
	/// * `position`: The position.
	/// * `cell_index`: The cell index.
	pub fn new(
		farmer_id: FarmerId,
		block_num: BlockNumber,
		position: Position,
		cell_index: u32,
	) -> Self {
		Self { block_num, farmer_id, cell_index, position, phantom: PhantomData }
	}

	/// Calculates the initial X value.
	pub fn calculate_initial_x(&self) -> u32 {
		utils::fold_hash::<H>(&self.hash())
	}

	/// Returns an iterator that generates a sequence of X values.
	///
	/// Parameters:
	/// * `count`: The number of X values to generate.
	///
	/// Returns:
	/// An iterator that yields tuples of X values and corresponding nonces.
	pub fn x_values_iterator(&self, count: u32) -> impl Iterator<Item = (u32, u8)> {
		let initial_x = self.calculate_initial_x();
		let key_bytes = initial_x.to_be_bytes();

		(0..count).scan(initial_x, move |state, _| {
			let binding = [*state as u8];
			let nonce = Nonce::from_slice(&binding);
			let key = Key::from_slice(&key_bytes);
			let mut cipher = ChaCha8::new(&key, &nonce);

			let mut buffer = state.to_le_bytes();
			cipher.apply_keystream(&mut buffer);
			*state = u32::from_le_bytes(buffer);

			Some((*state, nonce[0]))
		})
	}

	/// Calculates the final X value after a specified number of iterations.
	///
	/// Parameters:
	/// * `count`: The number of iterations to perform.
	///
	/// Returns:
	/// The final X value.
	pub fn calculate_final_x(&self, count: u32) -> u32 {
		self.x_values_iterator(count).last().unwrap().0
	}

	/// Generates a hash value based on the current state of `XValueManager`.
	pub fn hash(&self) -> H::Output {
		H::hash_of(self)
	}
}
