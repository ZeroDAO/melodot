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
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
pub use alloc::{
	vec::Vec,
	vec,
};

pub(crate) const DEFAULT_PREFIX: &[u8] = b"das_default_prefix";

pub mod traits;
#[cfg(feature = "sqlite")]
pub mod sqlite;
pub mod offchain;
#[cfg(feature = "outside")]
pub mod offchain_outside;
#[cfg(feature = "std")]
pub mod mock_db;