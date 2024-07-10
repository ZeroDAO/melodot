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

use crate::{traits::DasKv, Vec, DEFAULT_PREFIX};

use sp_core::offchain::StorageKind;

// Implementation for the non-outside environment
#[derive(Debug, Clone, Default)]
pub struct OffchainKv {
	prefix: Vec<u8>,
}

impl OffchainKv {
	fn get_prefixed_key(&self, key: &[u8]) -> Vec<u8> {
		let mut prefixed_key = self.prefix.clone();
		prefixed_key.extend_from_slice(key);
		prefixed_key
	}
}

impl DasKv for OffchainKv {
	fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
		let prefixed_key = self.get_prefixed_key(key);
		sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &prefixed_key)
	}

	fn set(&mut self, key: &[u8], value: &[u8]) {
		let prefixed_key = self.get_prefixed_key(key);
		sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &prefixed_key, value);
	}

	fn remove(&mut self, key: &[u8]) {
		let prefixed_key = self.get_prefixed_key(key);
		sp_io::offchain::local_storage_clear(StorageKind::PERSISTENT, &prefixed_key);
	}

	fn contains(&mut self, key: &[u8]) -> bool {
		self.get(key).is_some()
	}

	fn compare_and_set(&mut self, key: &[u8], old_value: Option<&[u8]>, new_value: &[u8]) -> bool {
		let prefixed_key = self.get_prefixed_key(key);
		let old_value = old_value.map(|v| v.to_vec());
		sp_io::offchain::local_storage_compare_and_set(
			StorageKind::PERSISTENT,
			&prefixed_key,
			old_value,
			new_value,
		)
	}
}

impl OffchainKv {
	pub fn new(prefix: Option<&[u8]>) -> Self {
		let prefix = prefix.unwrap_or(DEFAULT_PREFIX).to_vec();
		OffchainKv { prefix }
	}
}
