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
use melo_core_primitives::kzg::{BlsScalar,Segment};
use melo_core_primitives::config::{FIELD_ELEMENTS_PER_BLOB, SEGMENT_LENGTH};

pub fn segments_to_row(segments: &Vec<Segment>) -> [Option<BlsScalar>; 2 * FIELD_ELEMENTS_PER_BLOB] {
    // 检查长度是否小于 2 * FIELD_ELEMENTS_PER_BLOB
    // 检查是否有重复的 x
    // 检查所有的 y 是否相同
    let mut row = [None; FIELD_ELEMENTS_PER_BLOB * 2];
    for segment in segments.iter() {
        let index = segment.position.x * (SEGMENT_LENGTH as u32);
        for i in 0..SEGMENT_LENGTH {
            row[(index + (i as u32)) as usize] = Some(BlsScalar(segment.content[i]));
        }
    }
    row
}
