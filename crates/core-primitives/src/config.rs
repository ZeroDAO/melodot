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

pub const DAS_NETWORK_VERSION: &str = "0.0.1";
/// The maximum number of blocks that can be processed in a single call to `process_blocks`.
pub const BLOCK_SAMPLE_LIMIT: u32 = 3;
/// 允许提交不可用性区块的最长间隔区块数量
pub const MAX_UNAVAILABLE_BLOCK_INTERVAL: u32 = 3;