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

use erasure_coding::blob_to_poly;
use itertools::Itertools;
use melo_core_primitives::{
	config,
	kzg::{embedded_kzg_settings, Blob, KZG},
	segment::Segment,
};
use segment::poly_to_segment_vec;

#[cfg(test)]
mod tests;

extern crate alloc;

pub mod erasure_coding;
pub mod extend_col;
pub mod recovery;
pub mod segment;

pub fn bytes_vec_to_blobs(bytes_vec: &Vec<Vec<u8>>) -> Result<Vec<Blob>, String> {
	let blobs = bytes_vec
		.iter()
		.flat_map(|bytes| {
			bytes
				.chunks(config::BYTES_PER_BLOB)
				.map(|chunk| {
					Blob::try_from_bytes_pad(chunk)
						.expect("Failed to convert bytes to Blob; qed")
				})
				.collect_vec()
		})
		.collect_vec();
	Ok(blobs)
}

pub fn blobs_to_segments(blobs: &Vec<Blob>, chunk_size: usize) -> Result<Vec<Vec<Segment>>, String> {
	let kzg = KZG::new(embedded_kzg_settings());
	let matrix = blobs
		.iter()
		.enumerate()
		.map(|(y, blob)| {
			let poly = blob_to_poly(kzg.get_fs(),blob).expect("Failed to convert blob to poly; qed");
			poly_to_segment_vec(&poly, &kzg, y,chunk_size)
				.expect("Failed to convert poly to segment vector; qed")
		})
		.collect_vec();
    Ok(matrix)
}
