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

use itertools::Itertools;
use melo_core_primitives::{blob::Blob, kzg::SCALAR_SAFE_BYTES};

#[cfg(test)]
mod tests;

extern crate alloc;

pub mod erasure_coding;
pub mod extend_col;
pub mod recovery;
pub mod segment;

/// Converts a vector of byte vectors `bytes_vec` into a vector of `Blob`s, where each `Blob` contains 
/// `field_elements_per_blob` field elements.
///
/// # Arguments
///
/// * `bytes_vec` - A vector of byte vectors to convert.
/// * `field_elements_per_blob` - The number of field elements to include in each `Blob`.
///
/// # Returns
///
/// * `Result<Vec<Blob>, String>` - A vector of `Blob`s, or an error message if the conversion fails.
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
    bytes_vec: &Vec<Vec<u8>>,
    field_elements_per_blob: usize,
) -> Result<Vec<Blob>, String> {
    if bytes_vec.iter().any(|bytes| bytes.is_empty()) {
        return Err("bytes_vec should not contain empty bytes; qed".to_string());
    }
    if !field_elements_per_blob.is_power_of_two() {
        return Err("field_elements_per_blob should be a power of 2; qed".to_string());
    }
    if field_elements_per_blob == 0 {
        return Err("field_elements_per_blob should be greater than 0; qed".to_string());
    }
    let bytes_per_blob = SCALAR_SAFE_BYTES * field_elements_per_blob;
    let blobs = bytes_vec
        .iter()
        .flat_map(|bytes| {
            bytes
                .chunks(bytes_per_blob)
                .map(|chunk| {
                    Blob::try_from_bytes_pad(chunk, bytes_per_blob)
                        .expect("Failed to convert bytes to Blob; qed")
                })
                .collect_vec()
        })
        .collect_vec();
    Ok(blobs)
}
