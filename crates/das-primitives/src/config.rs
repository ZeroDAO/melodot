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

pub const BYTES_PER_FIELD_ELEMENT: usize = 32;
pub const EMBEDDED_KZG_SETTINGS_BYTES: &[u8] = include_bytes!("../../../scripts/eth-public-parameters-4096.bin");

pub const FIELD_ELEMENTS_PER_BLOB: usize = 2048;
pub const BYTES_PER_BLOB: usize = FIELD_ELEMENTS_PER_BLOB * BYTES_PER_FIELD_ELEMENT;