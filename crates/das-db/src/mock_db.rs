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

pub use crate::{traits::DasKv, vec, Vec};
pub use std::collections::HashMap;

pub struct MockDb {
	storage: HashMap<Vec<u8>, Vec<u8>>,
}

impl MockDb {
	pub fn new() -> Self {
		MockDb { storage: HashMap::new() }
	}
}
impl Default for MockDb {
	fn default() -> Self {
		Self::new()
	}
}

impl DasKv for MockDb {
	fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
		self.storage.get(key).cloned()
	}

	fn set(&mut self, key: &[u8], value: &[u8]) {
		self.storage.insert(key.to_vec(), value.to_vec());
	}

	fn remove(&mut self, key: &[u8]) {
		self.storage.remove(key);
	}

	fn contains(&mut self, key: &[u8]) -> bool {
		self.storage.contains_key(key)
	}

	fn compare_and_set(&mut self, key: &[u8], old_value: Option<&[u8]>, new_value: &[u8]) -> bool {
		match (self.get(key), old_value) {
			(Some(current_value), Some(old_value)) =>
				if current_value == old_value {
					self.set(key, new_value);
					true
				} else {
					false
				},
			(None, None) => {
				self.set(key, new_value);
				true
			},
			_ => false,
		}
	}
}
