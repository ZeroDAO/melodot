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

/// Selects indices from a hash where a specified number of consecutive bits are 1.
///
/// Parameters:
/// * `hash`: A reference to a 32-byte hash array.
/// * `start`: The starting index for the selection.
/// * `end`: The ending index for the selection.
/// * `n`: The number of consecutive 1 bits required.
///
/// Returns:
/// A vector of indices where each index meets the specified bit criteria.
pub fn select_indices(hash: &[u8; 32], start: usize, end: usize, n: usize) -> Vec<u32> {
	(start..end)
		.flat_map(|i| (0..8).map(move |bit| (i, bit))) // Generate index-bit pairs.
		.filter_map(|(i, bit)| {
			// Filter pairs where `n` consecutive bits are 1.
			if (0..n).all(|offset| {
				let next_bit_index = (bit + offset) % 8;
				let next_byte_index = (i + (bit + offset) / 8) % 32;

				hash[next_byte_index] & (1 << next_bit_index) != 0
			}) {
				Some((i * 8 + bit) as u32)
			} else {
				None
			}
		})
		.collect()
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

	pub use sp_core::H256;

	// #[test]
	// fn test_fold_hash_zero() {
	// 	let hash = H256::from([0u8; 32]);
	// 	let folded = fold_hash::<Keccak256>(&hash);
	// 	assert_eq!(folded, 0);
	// }

	// #[test]
	// fn test_fold_hash() {
	// 	let hash = H256::from([
	// 		50, 247, 15, 179, 42, 112, 214, 207, 137, 196, 15, 134, 193, 51, 85, 201, 156, 73, 1,
	// 		241, 92, 100, 240, 102, 244, 51, 148, 70, 49, 75, 53, 215,
	// 	]);
	// 	let folded = fold_hash::<Keccak256>(&hash);
	// 	assert_eq!(folded, 1428542261);
	// }

	// #[test]
	// fn test_fold_hash_alternating() {
	// 	let hash = H256::from([0xAAu8; 32]); // 10101010 repeated
	// 	let folded = fold_hash::<Keccak256>(&hash);
	// 	assert_eq!(folded, 0);
	// }

	// #[test]
	// fn test_fold_hash_max() {
	// 	let hash = H256::from([0xFFu8; 32]);
	// 	let folded = fold_hash::<Keccak256>(&hash);
	// 	assert_eq!(folded, 0);
	// }

	#[test]
	fn test_select_indices() {
		let hash = [0b1111_1111u8; 32];
		let indices = select_indices(&hash, 0, 32, 3);
		assert_eq!(indices.len(), 256);
	}

	#[test]
	fn test_select_indices_non_uniform() {
		let hash = [0b1010_1010u8; 32];
		let indices = select_indices(&hash, 0, 32, 2);
		assert!(indices.is_empty());
	}

	#[test]
	fn test_select_indices_no_match() {
		let hash = [0b0101_0101u8; 32];
		let indices = select_indices(&hash, 0, 32, 3);
		assert!(indices.is_empty());
	}

	#[test]
	fn test_select_indices_partial_bytes() {
		let hash = [0b1111_0000u8; 32];
		let indices = select_indices(&hash, 2, 30, 4);
		assert_eq!(
			indices,
			vec![
				20, 28, 36, 44, 52, 60, 68, 76, 84, 92, 100, 108, 116, 124, 132, 140, 148, 156,
				164, 172, 180, 188, 196, 204, 212, 220, 228, 236
			]
		);
	}
	#[test]
	fn test_is_index_valid_short_hash() {
		let hash = [0b1111_1111u8; 16]; // Shorter than 32 bytes
		assert!(!is_index_valid(&hash, 0, 16, 3));
	}

	#[test]
	fn test_is_index_valid_boundary() {
		let hash = [0b1111_1111u8; 32];
		assert!(is_index_valid(&hash, 7 * 8, 32, 1)); // Edge case at the boundary
	}

	#[test]
	fn test_is_index_valid_max_values() {
		let hash = [0b1111_1111u8; 32];
		assert!(!is_index_valid(&hash, 32 * 8 - 1, 32, 3)); // Edge case with maximum values
	}

	#[test]
	fn test_is_index_valid() {
		let hash = [0b1111_1111u8; 32];
		assert!(is_index_valid(&hash, 0, 32, 3));
		assert!(!is_index_valid(&hash, 256, 32, 3));
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
