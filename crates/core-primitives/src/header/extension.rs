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

use crate::{Decode, Encode, TypeInfo, Vec};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::RuntimeDebug;

/// The AppLookup struct represents information related to data in a block.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, TypeInfo, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AppLookup {
	/// The id of the app.
	#[codec(compact)]
	pub app_id: u32,
	/// The nonce of the app.
	pub nonce: u32,
	/// The count of the blobs.
	#[codec(compact)]
	pub count: u16,
}

impl AppLookup {
	/// Returns the lookup and the index of the lookup at the given position in the array of
	/// lookups.
	///
	/// # Arguments
	///
	/// * `lookups` - An array of lookups.
	/// * `at` - The position of the lookup to retrieve.
	///
	/// # Returns
	///
	/// * `Some((lookup, index))` - The lookup and the index of the lookup at the given position.
	/// * `None` - If the given position is out of bounds.
	pub fn get_lookup(lookups: &[Self], at: u32) -> Option<(&Self, u32)> {
		let mut prev_sum = 0u32;

		for lookup in lookups {
			let next_sum = prev_sum + lookup.count as u32;

			if at < next_sum {
				return Some((lookup, at - prev_sum))
			}

			prev_sum = next_sum;
		}

		None
	}
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, TypeInfo, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct HeaderExtension {
	/// The commitment of the data root.
	pub commitments_bytes: Vec<u8>,
	/// The lookup table for the data.
	pub app_lookup: Vec<AppLookup>,
}

/// HeaderExtension Struct
///
/// This struct represents a header extension and contains methods to manipulate it.
impl HeaderExtension {
    /// Creates a new instance of HeaderExtension.
    ///
    /// # Arguments
    ///
    /// * `commitments_bytes` - A vector of bytes representing the commitments.
    /// * `app_lookup` - A vector of `AppLookup` instances.
    ///
    /// # Returns
    ///
    /// A new instance of `HeaderExtension`.
    pub fn new(commitments_bytes: Vec<u8>, app_lookup: Vec<AppLookup>) -> Self {
        Self { commitments_bytes, app_lookup }
    }

    /// Returns the starting position of the given app ID and nonce.
    ///
    /// # Arguments
    ///
    /// * `app_id` - The ID of the app.
    /// * `nonce` - The nonce of the app.
    ///
    /// # Returns
    ///
    /// The starting position of the app if found, otherwise `None`.
    pub fn start_at(&self, app_id: u32, nonce: u32) -> Option<u32> {
        let mut sum = 0u32;

        for lookup in &self.app_lookup {
            if lookup.app_id == app_id && lookup.nonce == nonce {
                return Some(sum)
            }
            sum += lookup.count as u32;
        }

        None
    }

    /// Returns the `AppLookup` instance and its starting position at the given index.
    ///
    /// # Arguments
    ///
    /// * `at` - The index of the `AppLookup` instance.
    ///
    /// # Returns
    ///
    /// The `AppLookup` instance and its starting position if found, otherwise `None`.
    pub fn get_lookup(&self, at: u32) -> Option<(&AppLookup, u32)> {
        AppLookup::get_lookup(&self.app_lookup, at)
    }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_app_lookup_get_lookup() {
		let lookups = vec![
			AppLookup { app_id: 1, nonce: 1, count: 10 },
			AppLookup { app_id: 2, nonce: 1, count: 20 },
			AppLookup { app_id: 3, nonce: 1, count: 30 },
		];

		// at is within the range of the first AppLookup
		assert_eq!(
			AppLookup::get_lookup(&lookups, 5)
				.map(|(lookup, relative_row)| (lookup.app_id, relative_row)),
			Some((1, 5))
		);

		// at is the start of the second AppLookup
		assert_eq!(
			AppLookup::get_lookup(&lookups, 10)
				.map(|(lookup, relative_row)| (lookup.app_id, relative_row)),
			Some((2, 0))
		);

		// at is within the range of the second AppLookup
		assert_eq!(
			AppLookup::get_lookup(&lookups, 15)
				.map(|(lookup, relative_row)| (lookup.app_id, relative_row)),
			Some((2, 5))
		);

		// at is beyond the range of all AppLookups
		assert!(AppLookup::get_lookup(&lookups, 61).is_none());
	}

	#[test]
	fn test_header_extension_start_at() {
		let header_extension = HeaderExtension {
			commitments_bytes: vec![],
			app_lookup: vec![
				AppLookup { app_id: 1, nonce: 1, count: 10 },
				AppLookup { app_id: 2, nonce: 1, count: 20 },
			],
		};

		// Testing for existing app_id and nonce
		assert_eq!(header_extension.start_at(1, 1), Some(0));

		// Testing for app_id and nonce that are in the second AppLookup
		assert_eq!(header_extension.start_at(2, 1), Some(10));

		// Testing for non-existing app_id and nonce
		assert_eq!(header_extension.start_at(3, 1), None);
	}
}
