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

pub use melo_das_primitives::config::FIELD_ELEMENTS_PER_BLOB;

/// The current version of the network.
pub const DAS_NETWORK_VERSION: &str = "0.0.1";
/// The maximum number of blocks that can be processed in a single call to `process_blocks`.
pub const BLOCK_SAMPLE_LIMIT: u32 = 3;
/// The maximum interval of block numbers allowed for submitting unavailable blocks.
pub const MAX_UNAVAILABLE_BLOCK_INTERVAL: u32 = 3;
/// The number of elements per segment, must be a power of 2.
pub const FIELD_ELEMENTS_PER_SEGMENT: usize = 2usize.pow(4);
/// The number of samples/segments per blob.
pub const SEGMENTS_PER_BLOB: usize = FIELD_ELEMENTS_PER_BLOB / FIELD_ELEMENTS_PER_SEGMENT;
/// The number of segments per row after extension.
pub const EXTENDED_SEGMENTS_PER_BLOB: usize = SEGMENTS_PER_BLOB * 2;
/// Blocks with data available greater than this value.
pub const BLOCK_AVAILABILITY_THRESHOLD: u32 = 5;
/// The number of samples per block.
pub const SAMPLES_PER_BLOCK: usize = 8;