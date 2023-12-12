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

use crate::{
	crypto::{BlsScalar, KZGCommitment, KZGProof, Position, ReprConvert, KZG},
	polynomial::Polynomial,
};
use alloc::{
	string::{String, ToString},
	vec,
	vec::Vec,
};
use codec::{Decode, Encode};
use derive_more::{AsMut, AsRef, From};

/// This struct represents a segment of data with a position and content.
#[derive(Debug, Default, Decode, Encode, Clone, PartialEq, Eq, From, AsRef, AsMut)]
pub struct Segment {
	/// The position of the segment.
	pub position: Position,
	/// The content of the segment.
	pub content: SegmentData,
}

/// This struct represents the data of a segment with a vector of BlsScalar and a KZGProof.
#[derive(Decode, Encode, Debug, Default, Clone, PartialEq, Eq, From, AsRef, AsMut)]
pub struct SegmentData {
	/// The data of the segment.
	pub data: Vec<BlsScalar>,
	/// The KZGProof of the segment.
	pub proof: KZGProof,
}

impl SegmentData {
	/// This function creates a new SegmentData with a KZGProof and a size.
	pub fn new(proof: KZGProof, size: usize) -> Self {
		let arr = vec![BlsScalar::default(); size];
		Self { data: arr, proof }
	}

	/// This function returns the size of the data vector.
	pub fn size(&self) -> usize {
		self.data.len()
	}

	/// This function creates a new SegmentData from a Position, a vector of BlsScalar, a KZG, a
	/// Polynomial, and a chunk count.
	///
	/// It calculates the proof based on the given parameters and returns the `SegmentData`.
	pub fn from_data(
		positon: &Position,
		content: &[BlsScalar],
		kzg: &KZG,
		poly: &Polynomial,
		chunk_count: usize,
	) -> Result<Self, String> {
		kzg.compute_proof_multi(poly, positon.x as usize, chunk_count, content.len())
			.map(|p| Ok(Self { data: content.to_vec(), proof: p }))?
	}
}

impl Segment {
	/// This function creates a new `Segment` with a `Position`, a vector of `BlsScalar`, and a
	/// `KZGProof`.
	pub fn new(position: Position, data: &[BlsScalar], proof: KZGProof) -> Self {
		let segment_data = SegmentData { data: data.to_vec(), proof };
		Self { position, content: segment_data }
	}

	/// This function returns the size of the data vector in the `SegmentData` of the `Segment`.
	pub fn size(&self) -> usize {
		self.content.data.len()
	}

	/// This function checks if the data vector is valid and returns a Result.
	pub fn checked(&self) -> Result<Self, String> {
		if self.content.data.is_empty() {
			return Err("segment data is empty".to_string())
		}
		// data.len() is a power of two
		if !self.content.data.len().is_power_of_two() {
			return Err("segment data length is not a power of two".to_string())
		}
		Ok(self.clone())
	}

	/// This function verifies the proof of the `Segment` using a `KZG`, a `KZGCommitment`, and a
	/// count.
	///
	/// It returns a `Result` with a boolean indicating if the proof is valid or an error message.
	pub fn verify(
		&self,
		kzg: &KZG,
		commitment: &KZGCommitment,
		count: usize,
	) -> Result<bool, String> {
		kzg.check_proof_multi(
			commitment,
			self.position.x as usize,
			count,
			&BlsScalar::vec_to_repr(self.content.data.clone()),
			&self.content.proof,
			self.size(),
		)
	}
}
