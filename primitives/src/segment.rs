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
extern crate alloc;

use alloc::{string::String, vec::Vec};
use derive_more::{AsMut, AsRef, From};
use rust_kzg_blst::types::fr::FsFr;

use crate::config::{FIELD_ELEMENTS_PER_BLOB, SEGMENT_LENGTH};
use crate::kzg::{KZG, KZGCommitment, KZGProof, Positon, Cell};

#[derive(Debug, Default, Clone, PartialEq, Eq, From, AsRef, AsMut)]
pub struct Segment {
	pub position: Positon,
	pub content: [FsFr; SEGMENT_LENGTH],
	pub proof: KZGProof,
}

impl Segment {
	pub fn new(position: Positon, content: &[FsFr], proof: KZGProof) -> Self {
		// 检查 content 长度是否为 16

		let mut arr = [FsFr::default(); SEGMENT_LENGTH];
		arr.copy_from_slice(&content[..SEGMENT_LENGTH]);
		Self { position, content: arr, proof }
	}

	pub fn verify(&self, kzg: &KZG, commitment: &KZGCommitment) -> Result<bool, String> {
		let domain_stride = kzg.max_width() / (2 * FIELD_ELEMENTS_PER_BLOB);
		let chunk_count = FIELD_ELEMENTS_PER_BLOB / SEGMENT_LENGTH;
		let domain_pos = Self::reverse_bits_limited(chunk_count, self.position.x as usize);
		let x = kzg.fs.get_expanded_roots_of_unity_at(domain_pos * domain_stride);
		kzg.ks.check_proof_multi(
			&commitment.0,
			&self.proof,
			&x,
			&self.content,
			FIELD_ELEMENTS_PER_BLOB,
		)
	}

	fn reverse_bits_limited(length: usize, value: usize) -> usize {
		let unused_bits = length.leading_zeros();
		value.reverse_bits() >> unused_bits
	}

	pub fn get_cell_by_offset(&self, offset: usize) -> Cell {
		let x = self.position.x * (SEGMENT_LENGTH as u32) + (offset as u32);
		let position = Positon { x, y: self.position.y };
		Cell { data: self.content[offset], position }
	}

	pub fn get_cell_by_index(&self, index: usize) -> Cell {
		let offset = index % SEGMENT_LENGTH;
		self.get_cell_by_offset(offset)
	}

	pub fn get_all_cells(&self) -> Vec<Cell> {
		let mut cells = Vec::with_capacity(SEGMENT_LENGTH);
		for i in 0..SEGMENT_LENGTH {
			cells.push(self.get_cell_by_offset(i));
		}
		cells
	}
}
