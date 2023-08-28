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

use sp_core::offchain::StorageKind;
use crate::Vec;

pub fn save_to_localstorage_with_prefix(key: &[u8], value: &[u8], prefix: &[u8]) {
	let mut prefixed_key = prefix.to_vec();
	prefixed_key.extend_from_slice(key);
	sp_io::offchain::local_storage_set(StorageKind::PERSISTENT, &prefixed_key, value);
}

pub fn get_from_localstorage_with_prefix(key: &[u8], prefix: &[u8]) -> Option<Vec<u8>> {
	let mut prefixed_key = prefix.to_vec();
	prefixed_key.extend_from_slice(key);
	sp_io::offchain::local_storage_get(StorageKind::PERSISTENT, &prefixed_key)
}
