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
use crate::Vec;
#[cfg(feature = "std")]
use bit_vec::BitVec;

/// Folds a hash output to a 32-bit unsigned integer.
/// This function is useful for reducing a larger hash output to a smaller, manageable size.
///
/// Parameters:
/// * `hash`: A reference to the hash output.
///
/// Returns:
/// A 32-bit unsigned integer representing the folded hash.
pub fn fold_hash(hash: &[u8]) -> u32 {
	let (first_half, second_half) = hash.split_at(16); // Split the hash into two halves.

	let mut folded = 0u32;
	for i in 0..4 {
		// Process each chunk of 4 bytes from both halves to compute the folded value.
		let first_half_chunk = u32::from_be_bytes(first_half[4 * i..4 * i + 4].try_into().unwrap());
		let second_half_chunk =
			u32::from_be_bytes(second_half[4 * i..4 * i + 4].try_into().unwrap());
		folded ^= first_half_chunk ^ second_half_chunk; // XOR the chunks together.
	}

	folded
}

/// Computes a 16-bit hash from a byte array using XOR folding.
///
/// This function takes a slice of bytes (`&[u8]`) and computes a 16-bit unsigned integer
/// by performing an XOR operation on each pair of bytes. The XOR folding technique is used
/// to condense the input byte array into a 16-bit hash value.
///
/// # Arguments
///
/// * `hash` - A byte slice (`&[u8]`) representing the input data to be hashed.
///
/// # Returns
///
/// A 16-bit unsigned integer (`u16`) representing the XOR folded hash of the input data.
///
/// # Notes
///
/// - The function iterates over the byte slice in chunks of two bytes.
/// - Each pair of bytes is converted to a `u16` and then XORed with the accumulator.
/// - If the number of bytes in the slice is odd, the last byte is XORed with 0.
pub fn hash_to_u16_xor(hash: &[u8]) -> u16 {
	hash.chunks(2).fold(0u16, |acc, chunk| {
		acc ^ u16::from_be_bytes([chunk[0], chunk.get(1).cloned().unwrap_or(0)])
	})
}

#[cfg(feature = "std")]
pub fn hash_to_bitvec(hash: &[u8; 32]) -> BitVec {
	BitVec::from_bytes(hash)
}

/// Selects indices where a specified number of consecutive bits are 1.
///
/// This function scans a hash represented as a byte array and finds indices
/// where 'n' consecutive bits are set to 1. The search is performed within
/// the range specified by `start` and `end`. The function supports wrapping
/// around the end of the array to the start, enabling circular checks.
///
/// # Parameters
/// * `hash`: A reference to a 32-byte hash array.
/// * `start`: The starting index for the selection within the bit array.
/// * `end`: The end index (exclusive) for the selection within the bit array.
/// * `n`: The number of consecutive 1 bits required.
///
/// # Returns
/// A vector of indices where each index meets the specified bit criteria.
/// If `n` is 0, or if `start` or `end` are out of bounds, the function returns an empty vector.
#[cfg(feature = "std")]
pub fn select_indices(hash: &[u8; 32], start: usize, end: usize, n: usize) -> Vec<u32> {
	let bits = hash_to_bitvec(hash);
	let bit_len = bits.len();

	if n == 0 || n > bit_len || start >= end || end > bit_len {
		return Vec::new()
	}

	let mut indices = Vec::new();

	for i in start..end {
		let mut all_ones = true;
		for j in 0..n {
			if !bits[(i + j) % bit_len] {
				all_ones = false;
				break
			}
		}

		if all_ones {
			indices.push(i as u32);
		}
	}

	indices
}

/// Checks if a given index is valid based on the number of consecutive 1 bits in a hash.
///
/// Parameters:
/// * `hash`: A byte slice reference representing the hash.
/// * `index`: The index to check.
/// * `max_index`: The maximum allowed index.
/// * `n`: The number of consecutive 1 bits required.
///
/// Returns:
/// `true` if the index is valid, otherwise `false`.
pub fn is_index_valid(hash: &[u8], index: usize, max_index: usize, n: usize) -> bool {
	if hash.len() < 32 || index >= max_index * 8 {
		return false
	}

	if index + n > max_index * 8 {
		return false
	}

	let byte_index = index / 8;
	let bit_index = index % 8;

	(0..n).all(|offset| {
		let next_bit_index = (bit_index + offset) % 8;
		let next_byte_index = byte_index + (bit_index + offset) / 8;

		if next_byte_index >= hash.len() {
			return false
		}

		hash[next_byte_index] & (1 << next_bit_index) != 0
	})
}

/// Validates if the leading bits of the data are zeros as specified by the `zeros` parameter.
///
/// Parameters:
/// * `data`: A byte slice reference.
/// * `zeros`: The number of leading zeros required.
///
/// Returns:
/// `true` if the leading bits are zeros as required, otherwise `false`.
pub fn validate_leading_zeros(data: &[u8], zeros: u32) -> bool {
	if data.is_empty() {
		return false
	}

	let zero_bytes = zeros as usize / 8; // Number of whole zero bytes.
	let zero_bits = zeros as usize % 8; // Remaining zero bits.

	// Check if the whole zero bytes are all zeros.
	if !data.iter().take(zero_bytes).all(|&b| b == 0) {
		return false
	}

	// Check the remaining zero bits, if there are any.
	if zero_bits > 0 {
		data.get(zero_bytes).map_or(false, |&b| b.leading_zeros() as usize >= zero_bits)
	} else {
		true
	}
}

/// Performs a bitwise exclusive OR (XOR) operation on two byte slices.
///
/// Parameters:
/// * `a`: The first byte slice.
/// * `b`: The second byte slice.
///
/// Returns:
/// A `Vec<u8>` containing the result of the XOR operation.
pub fn xor_byte_slices(a: &[u8], b: &[u8]) -> Vec<u8> {
	let (shorter, longer) = if a.len() > b.len() { (b, a) } else { (a, b) };

	let xor_part: Vec<u8> = shorter.iter().zip(longer.iter()).map(|(&x, &y)| x ^ y).collect();
	let remaining_part = &longer[shorter.len()..];

	[xor_part.as_slice(), remaining_part].concat()
}

#[cfg(test)]
mod tests {

	use super::*;
	// use crate::BlakeTwo256;
	// use sp_core::Hasher;
	pub use sp_core::H256;

	// #[test]
	// fn find_y_collisions() {
	// 	let target_hashes = [58239u16];
	// 	let mut input = 0u64;

	// 	loop {
	// 		let hash = BlakeTwo256::hash(&input.to_be_bytes());
	// 		let hash_bytes = hash.as_ref();
	// 		let result = hash_to_u16_xor(hash_bytes);

	// 		if target_hashes.contains(&result) {
	// 			println!("Found a match for {}: input = {:?}", result, hash_bytes);
	// 			break
	// 		}

	// 		input += 1;
	// 	}
	// }

	#[test]
	fn test_fold_hash() {
		let hash = [
			50, 247, 15, 179, 42, 112, 214, 207, 137, 196, 15, 134, 193, 51, 85, 201, 156, 73, 1,
			241, 92, 100, 240, 102, 244, 51, 148, 70, 49, 75, 53, 215,
		];
		let folded = fold_hash(&hash);
		assert_eq!(folded, 1428542261);
	}

	#[test]
	fn test_select_indices_basic() {
		let hash = [0b10101010; 32];
		let indices = select_indices(&hash, 0, 256, 2);
		assert!(indices.is_empty());

		let indices = select_indices(&hash, 0, 256, 1);
		let expected_indices: Vec<u32> = (0..255).step_by(2).collect();
		assert_eq!(indices, expected_indices);
	}

	#[test]
	fn test_select_indices_n_too_large() {
		let hash = [0b11111111; 32];
		let indices = select_indices(&hash, 0, 258, 33);
		assert!(indices.is_empty());
	}

	#[test]
	fn test_select_indices_wraparound() {
		let mut hash = [0; 32];
		hash[0] = 0b10000000;
		hash[31] = 0b00000001;
		let indices = select_indices(&hash, 0, 256, 2);
		assert_eq!(indices, vec![255]);
	}

	#[test]
	fn test_select_indices_boundary_conditions() {
		let hash = [0b11110000; 32];
		let indices = select_indices(&hash, 0, 32 * 8, 4);
		let expected_indices = (0..32).map(|x| x * 8).collect::<Vec<u32>>();
		assert_eq!(indices, expected_indices);
	}

	#[test]
	fn test_select_indices_consecutive_ones() {
		let mut hash = [0; 32];
		hash[1] = 0b00001111;
		hash[2] = 0b11110000;
		let indices = select_indices(&hash, 8, 24, 8);
		assert_eq!(indices, vec![12]);
	}

	#[test]
	fn test_select_indices_empty() {
		let hash = [0; 32];
		let indices = select_indices(&hash, 0, 256, 1);
		assert!(indices.is_empty());
	}

	#[test]
	fn test_select_indices_invalid_n() {
		let hash = [0b10101010; 32];
		let indices = select_indices(&hash, 0, 256, 0);
		assert!(indices.is_empty());
	}

	#[test]
	fn test_select_indices_out_of_bounds() {
		let hash = [0b10101010; 32];
		let indices = select_indices(&hash, 256, 512, 2);
		assert!(indices.is_empty());
	}

	#[test]
	fn test_validate_leading_zeros() {
		let data = [0u8; 32];
		assert!(validate_leading_zeros(&data, 32 * 8));
		assert!(!validate_leading_zeros(&data, 32 * 8 + 1));
	}

	#[test]
	fn test_validate_leading_zeros_exact_zeros() {
		let data = [0x00, 0x00, 0x01, 0xFF]; // 16 leading zeros
		assert!(validate_leading_zeros(&data, 16));
	}

	#[test]
	fn test_validate_leading_zeros_not_enough_zeros() {
		let data = [0x00, 0x01, 0xFF, 0xFF]; // Only 8 leading zeros
		assert!(!validate_leading_zeros(&data, 16));
	}

	#[test]
	fn test_validate_leading_zeros_empty_data() {
		let data: [u8; 0] = [];
		assert!(!validate_leading_zeros(&data, 8));
	}

	#[test]
	fn test_xor_byte_slices() {
		let a = [0xFFu8; 32];
		let b = [0x00u8; 32];
		let result = xor_byte_slices(&a, &b);
		assert_eq!(result, a);
	}

	#[test]
	fn test_xor_byte_slices_one_zero_slice() {
		let a = [0xFFu8; 32];
		let b = [0x00u8; 32];
		let result = xor_byte_slices(&a, &b);
		assert_eq!(result, a);
	}

	#[test]
	fn test_xor_byte_slices_unequal_length() {
		let a = [0xFFu8; 16];
		let b = [0x00u8; 32];
		let result = xor_byte_slices(&a, &b);
		assert_eq!(
			result,
			[
				255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0,
				0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
			]
		);
	}

	#[test]
	fn test_xor_byte_slices_alternating() {
		let a = [0xAAu8; 32]; // 10101010 repeated
		let b = [0x55u8; 32]; // 01010101 repeated
		let result = xor_byte_slices(&a, &b);
		assert_eq!(result, [0xFFu8; 32]);
	}
}
