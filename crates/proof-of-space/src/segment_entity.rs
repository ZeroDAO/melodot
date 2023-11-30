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

use crate::{get_record_id, DasKv, Encode, Decode, FarmerId, Segment, Table};
use core::marker::PhantomData; // Used for type safety without storing actual data.
use sp_runtime::traits::Hash as HashT; // Importing Hash trait for cryptographic operations.

/// `SegmentEntity` represents an entity in a segment with a generic database and hash type.
///
/// Type parameters:
/// * `DB`: The type of the database that implements `DasKv`.
/// * `H`: A hash type that implements `HashT`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SegmentEntity<DB, H: HashT> {
    pub segment: Segment,
    _db: PhantomData<DB>,
    _hash: PhantomData<H>,
}

impl<DB: DasKv, H: HashT> SegmentEntity<DB, H> {
    /// Saves the `Segment` to the database.
    ///
    /// Parameters:
    /// * `db`: A mutable reference to the database.
    /// * `record_id`: A byte slice representing the record identifier.
    ///
    /// This method encodes the `Segment` and stores it in the database using the provided record ID.
    pub fn save(&self, db: &mut DB, record_id: &[u8]) {
        let value = self.segment.encode();
        db.set(record_id, &value);
    }

    /// Saves the `Segment` and its cells to the database.
    ///
    /// Parameters:
    /// * `db`: A mutable reference to the database.
    /// * `block_num`: The block number associated with the segment.
    /// * `farmer_id`: A reference to the farmer's ID.
    ///
    /// This method calculates the record ID, iterates over cells to save them,
    /// and then saves the segment itself.
    pub fn save_with_cells<BlockNumber>(&self, db: &mut DB, block_num: BlockNumber, farmer_id: &FarmerId)
    where
        BlockNumber: Clone + sp_std::hash::Hash + Encode + Decode,
    {
        let record_id = get_record_id(&self.segment.position, &block_num);

        let table = Table::<H, BlockNumber>::new(record_id);
        for (final_x, cell_metadata) in table.cells(&self.segment, farmer_id, block_num) {
            cell_metadata.save(db, final_x);
        }

        self.save(db, &record_id.as_bytes());
    }
}
