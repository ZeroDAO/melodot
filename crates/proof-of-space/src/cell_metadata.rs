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

use crate::{DasKv, Decode, Encode, H256};

/// `CellMetadata` is a struct representing the metadata of a cell.
/// It includes the `record_id` and an index to uniquely identify a cell.
///
/// Fields:
/// * `record_id`: A `Vec<u8>` that stores the unique identifier of the cell.
/// * `index`: A `u32` that represents an index or position of the cell.
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq, Encode, Decode)]
pub struct CellMetadata {
    pub record_id: Vec<u8>,
    pub index: u32,
}

impl CellMetadata {
    /// Constructs a new `CellMetadata` with the given `record_id` and a default index of 0.
    ///
    /// Parameters:
    /// * `record_id`: A `H256` hash that represents the unique identifier for the cell.
    ///
    /// Returns:
    /// A new `CellMetadata` instance with `record_id` and `index` set to 0.
    pub fn new(record_id: H256) -> Self {
        Self { 
            record_id: record_id.as_ref().into(), // Converts H256 to Vec<u8>
            index: 0,
        }
    }

    /// Saves the `CellMetadata` to a given database.
    /// It serializes the `CellMetadata` and stores it with the specified key.
    ///
    /// Parameters:
    /// * `db`: A mutable reference to an object that implements the `DasKv` trait.
    /// * `final_x`: A `u32` that is used as the key for storage.
    ///
    /// Note:
    /// The function uses `final_x` to generate the key and serializes `self` as the value.
    pub fn save(&self, db: &mut impl DasKv, final_x: u32) {
        let key = final_x.to_be_bytes(); // Converts the key to big-endian bytes
        let value = self.encode(); // Encodes the CellMetadata for storage
        db.set(&key, &value);
    }
}

