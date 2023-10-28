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

#[derive(PartialEq, Eq, Clone, RuntimeDebug, TypeInfo, Encode, Decode, Default, Serialize, Deserialize)]
pub struct AppLookup {
	#[codec(compact)]
	pub app_id: u32,
	#[codec(compact)]
	pub nonce: u32,
	#[codec(compact)]
	pub count: u16,
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, TypeInfo, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct HeaderExtension {
	/// The commitment of the data root.
	pub commitments_bytes: Vec<u8>,
	pub app_lookup: Vec<AppLookup>,
}

impl HeaderExtension {
	pub fn new(commitments_bytes: Vec<u8>, app_lookup: Vec<AppLookup>) -> Self {
		Self {
			commitments_bytes,
			app_lookup,
		}
	}

    pub fn start_at(&self, app_id: u32, nonce: u32) -> Option<u32> {
        let mut sum = 0u32;

        for lookup in &self.app_lookup {
            if lookup.app_id == app_id && lookup.nonce == nonce {
                return Some(sum);
            }
            sum += lookup.count as u32;
        }

        None
    }

    pub fn get_lookup(&self, at: u32) -> Option<&AppLookup> {
        let mut sum = 0u32;
        
        for lookup in &self.app_lookup {
            sum += lookup.count as u32;

            if at < sum {
                return Some(lookup);
            }
        }

        None
    }
}
