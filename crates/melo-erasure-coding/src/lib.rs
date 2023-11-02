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

use melo_das_primitives::{blob::Blob, crypto::SCALAR_SAFE_BYTES, KZG};

#[cfg(test)]
mod tests;

extern crate alloc;
pub use alloc::{
	string::{String, ToString},
	vec,
	vec::Vec,
};
use segment::poly_to_segment_vec;

pub mod erasure_coding;
pub mod extend_col;
pub mod recovery;
pub mod segment;

/// Converts a vector of byte vectors `bytes_vec` into a vector of `Blob`s, where each `Blob`
/// contains `field_elements_per_blob` field elements.
///
/// # Arguments
///
/// * `bytes_vec` - A vector of byte vectors to convert.
/// * `field_elements_per_blob` - The number of field elements to include in each `Blob`.
///
/// # Returns
///
/// * `Result<Vec<Blob>, String>` - A vector of `Blob`s, or an error message if the conversion
///   fails.
///
/// # Errors
///
/// Returns an error message if:
///
/// * `bytes_vec` contains an empty byte vector.
/// * `field_elements_per_blob` is not a power of two.
/// * `field_elements_per_blob` is zero.
///
/// # Notes
///
/// * If the input byte vectors do not contain enough bytes to create a `Blob`, the remaining bytes
/// are padded with zeroes. When using this function, the final recovered data should be determined
/// based on the length of the original data.
pub fn bytes_vec_to_blobs(
	bytes_vec: &[Vec<u8>],
	field_elements_per_blob: usize,
) -> Result<Vec<Blob>, String> {
	if bytes_vec.iter().any(|bytes| bytes.is_empty()) {
		return Err("bytes_vec should not contain empty bytes; qed".to_string())
	}

	let bytes_per_blob = get_bytes_per_blob(field_elements_per_blob)?;
	let blobs = bytes_vec
		.iter()
		.flat_map(|bytes| {
			bytes
				.chunks(bytes_per_blob)
				.map(|chunk| {
					Blob::try_from_bytes_pad(chunk, bytes_per_blob)
						.expect("Failed to convert bytes to Blob; qed")
				})
				.collect::<Vec<_>>()
		})
		.collect();
	Ok(blobs)
}

/// Converts a `bytes` into a vector of `Blob`s, where each `Blob` contains
/// `field_elements_per_blob` field elements.
///
/// # Arguments
///
/// * `bytes` - A vector of bytes to convert.
/// * `field_elements_per_blob` - The number of field elements to include in each `Blob`.
///
/// # Returns
///
/// * `Result<Vec<Blob>, String>` - A vector of `Blob`s, or an error message if the conversion
///   fails.
///
/// # Errors
///
/// Returns an error message if:
///
/// * `bytes` is an empty byte vector.
/// * `field_elements_per_blob` is not a power of two.
/// * `field_elements_per_blob` is zero.
///
/// # Notes
///
/// * If the input `bytes` do not contain enough bytes to create a `Blob`, the remaining bytes
/// are padded with zeroes. When using this function, the final recovered data should be determined
/// based on the length of the original data.
// TODO: test
pub fn bytes_to_blobs(bytes: &[u8], field_elements_per_blob: usize) -> Result<Vec<Blob>, String> {
	if bytes.is_empty() {
		return Err("bytes should not contain empty bytes; qed".to_string())
	}
	let bytes_per_blob = get_bytes_per_blob(field_elements_per_blob)?;
	let blobs = bytes
		.chunks(bytes_per_blob)
		.map(|chunk| {
			Blob::try_from_bytes_pad(chunk, bytes_per_blob)
				.expect("Failed to convert bytes to Blob; qed")
		})
		.collect();
	Ok(blobs)
}

pub fn bytes_to_segments(
	bytes: &[u8],
	field_elements_per_blob: usize,
	field_elements_per_segment: usize,
	kzg: &KZG,
) -> Result<Vec<melo_das_primitives::Segment>, String> {
	if bytes.is_empty() {
		return Err("bytes should not contain empty bytes; qed".to_string())
	}
	let bytes_per_blob = get_bytes_per_blob(field_elements_per_blob)?;
	let segments = bytes
		.chunks(bytes_per_blob)
		.enumerate()
		.flat_map(|(y, chunk)| {
			let ploy = Blob::try_from_bytes_pad(chunk, bytes_per_blob)
				.expect("Failed to convert bytes to Blob; qed")
				.to_poly();
			poly_to_segment_vec(&ploy, kzg, y, field_elements_per_segment)
				.expect("Failed to convert bytes to Blob; qed")
		})
		.collect::<Vec<_>>();
	Ok(segments)
}

fn get_bytes_per_blob(field_elements_per_blob: usize) -> Result<usize, String> {
	let bytes_per_blob = SCALAR_SAFE_BYTES * field_elements_per_blob;
	if !field_elements_per_blob.is_power_of_two() {
		return Err("field_elements_per_blob should be a power of 2; qed".to_string())
	}
	if field_elements_per_blob == 0 {
		return Err("field_elements_per_blob should be greater than 0; qed".to_string())
	}
	Ok(bytes_per_blob)
}
