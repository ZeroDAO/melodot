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

use core::marker::PhantomData;

use crate::traits::DasKv;

use sc_client_api::Backend;
use sc_offchain::OffchainDb;
use sp_core::offchain::DbExternalities;
use sp_runtime::traits::Block;

use sp_core::offchain::StorageKind;

const DEFAULT_PREFIX: &[u8] = b"das_default_prefix";

#[derive(Debug, Clone)]
pub struct OffchainKvOutside<B: Block, BE: Backend<B>> {
    db: OffchainDb<BE::OffchainStorage>,
    prefix: Vec<u8>,
    _phantom_b: PhantomData<B>,
    _phantom_be: PhantomData<BE>,
}

impl<B: Block, BE: Backend<B>> OffchainKvOutside<B, BE> {
    fn get_prefixed_key(&self, key: &[u8]) -> Vec<u8> {
        let mut prefixed_key = self.prefix.clone();
        prefixed_key.extend_from_slice(key);
        prefixed_key
    }

    // 注意这里需要接受 DB 类型作为参数而不是 OffchainDb<BE::OffchainStorage>
    pub fn new(db: OffchainDb<BE::OffchainStorage>, prefix: Option<&[u8]>) -> Self {
        let prefix = prefix.unwrap_or_else(|| DEFAULT_PREFIX).to_vec();
        OffchainKvOutside {
            db,
            prefix,
            _phantom_b: PhantomData,
            _phantom_be: PhantomData,
        }
    }
}

impl<B: Block, BE: Backend<B>> DasKv for OffchainKvOutside<B, BE> {
	fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
		let prefixed_key = self.get_prefixed_key(key);
		self.db.local_storage_get(StorageKind::PERSISTENT, &prefixed_key)
	}

	fn set(&mut self, key: &[u8], value: &[u8]) {
		let prefixed_key = self.get_prefixed_key(key);
		self.db.local_storage_set(StorageKind::PERSISTENT, &prefixed_key, value);
	}

	fn remove(&mut self, key: &[u8]) {
		let prefixed_key = self.get_prefixed_key(key);
		self.db.local_storage_clear(StorageKind::PERSISTENT, &prefixed_key);
	}

	fn contains(&mut self, key: &[u8]) -> bool {
		self.get(key).is_some()
	}

	fn compare_and_set(&mut self, key: &[u8], old_value: Option<&[u8]>, new_value: &[u8]) -> bool {
		let prefixed_key = self.get_prefixed_key(key);
		self.db.local_storage_compare_and_set(
			StorageKind::PERSISTENT,
			&prefixed_key,
			old_value,
			new_value,
		)
	}
}
