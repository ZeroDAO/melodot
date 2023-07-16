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

extern crate alloc;

use alloc::sync::Arc;
use rust_kzg_blst::types::fft_settings::FsFFTSettings;

#[derive(Debug, Clone)]
pub struct ErasureCoding {
    fft_settings: Arc<FsFFTSettings>,
}

impl ErasureCoding {
	pub fn new(fft_settings: Arc<FsFFTSettings>) -> Self {
		Self { fft_settings }
	}

}
