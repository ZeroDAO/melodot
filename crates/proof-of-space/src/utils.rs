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

use sp_runtime::traits::Hash;

/// Folds a hash output to a 32-bit unsigned integer.
/// This function is useful for reducing a larger hash output to a smaller, manageable size.
///
/// Parameters:
/// * `hash`: A reference to the hash output.
///
/// Returns:
/// A 32-bit unsigned integer representing the folded hash.
pub fn fold_hash<H: Hash>(hash: &H::Output) -> u32 {
    let hash_bytes = hash.as_ref(); // Convert the hash output to a byte array.
    let (first_half, second_half) = hash_bytes.split_at(16); // Split the hash into two halves.

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
pub fn select_indices(hash: &[u8; 32], start: usize, end: usize, n: usize) -> Vec<usize> {
    (start..end)
        .flat_map(|i| (0..8).map(move |bit| (i, bit))) // Generate index-bit pairs.
        .filter_map(|(i, bit)| {
            // Filter pairs where `n` consecutive bits are 1.
            if (0..n).all(|offset| {
                let next_bit_index = (bit + offset) % 8;
                let next_byte_index = (i + (bit + offset) / 8) % 32;

                hash[next_byte_index] & (1 << next_bit_index) != 0
            }) {
                Some(i * 8 + bit)
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
        return false; // Check if the hash length and index are within bounds.
    }

    let byte_index = index / 8;
    let bit_index = index % 8;

    (0..n).all(|offset| {
        // Check for `n` consecutive 1 bits.
        let next_bit_index = (bit_index + offset) % 8;
        let next_byte_index = (byte_index + (bit_index + offset) / 8) % 32;

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
    let zero_bytes = zeros as usize / 8; // Number of whole zero bytes.
    let zero_bits = zeros as usize % 8;  // Remaining zero bits.

    data.iter().take(zero_bytes).all(|&b| b == 0) // Check the whole zero bytes.
        && data.get(zero_bytes).map_or(false, |&b| b.leading_zeros() as usize >= zero_bits) // Check the remaining zero bits.
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
    a.iter().zip(b.iter()).map(|(&x, &y)| x ^ y).collect() // XOR each pair of bytes.
}
