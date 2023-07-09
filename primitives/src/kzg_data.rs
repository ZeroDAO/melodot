// Copyright 2023 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this filepub(crate)  except in compliance with the License.
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

use alloc::{
	string::{String, ToString},
	vec::Vec,
};
use core::hash::{Hash, Hasher};
use core::mem;
use derive_more::{AsMut, AsRef, Deref, DerefMut, From, Into};
use kzg::{Fr, G1};
use parity_scale_codec::{Decode, Encode, EncodeLike, Input, MaxEncodedLen};
use rust_kzg_blst::types::{fr::FsFr, g1::FsG1};
use scale_info::{Type, TypeInfo};

macro_rules! kzg_data_size {
	($name:ident, $type:ty, $size:expr) => {
		#[derive(
			Debug, Default, Copy, Clone, PartialEq, Eq, Into, From, AsRef, AsMut, Deref, DerefMut,
		)]
		#[repr(transparent)]
		pub struct $name(pub $type);
		impl $name {
			const SIZE: usize = $size;

			#[inline]
			pub fn to_bytes(&self) -> [u8; $size] {
				self.to_bytes()
			}

			#[inline]
			pub fn try_from_bytes(bytes: &[u8; $size]) -> Result<Self, String> {
				Ok($name(<$type>::from_bytes(bytes)?))
			}
		}

		impl Hash for $name {
			fn hash<H: Hasher>(&self, state: &mut H) {
				self.to_bytes().hash(state);
			}
		}

		impl From<$name> for [u8; $size] {
			#[inline]
			fn from(kzd_data: $name) -> Self {
				kzd_data.to_bytes()
			}
		}

		impl From<&$name> for [u8; $size] {
			#[inline]
			fn from(kzd_data: &$name) -> Self {
				kzd_data.to_bytes()
			}
		}

		impl TryFrom<&[u8; $size]> for $name {
			type Error = String;

			#[inline]
			fn try_from(bytes: &[u8; $size]) -> Result<Self, Self::Error> {
				Self::try_from_bytes(bytes)
			}
		}

		impl TryFrom<[u8; $size]> for $name {
			type Error = String;

			#[inline]
			fn try_from(bytes: [u8; $size]) -> Result<Self, Self::Error> {
				Self::try_from(&bytes)
			}
		}

		impl Encode for $name {
			#[inline]
			fn size_hint(&self) -> usize {
				$size
			}

			fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
				f(&self.to_bytes())
			}

			#[inline]
			fn encoded_size(&self) -> usize {
				$size
			}
		}

		impl EncodeLike for $name {}

		impl MaxEncodedLen for $name {
			#[inline]
			fn max_encoded_len() -> usize {
				$size
			}
		}

		impl Decode for $name {
			fn decode<I: Input>(input: &mut I) -> Result<Self, parity_scale_codec::Error> {
				Self::try_from_bytes(&Decode::decode(input)?).map_err(|error| {
					parity_scale_codec::Error::from("Failed to decode from bytes")
						.chain(alloc::format!("{error:?}"))
				})
			}

			#[inline]
			fn encoded_fixed_size() -> Option<usize> {
				Some($size)
			}
		}

		impl TypeInfo for $name {
			type Identity = Self;

			fn type_info() -> Type {
				Type::builder()
					.path(scale_info::Path::new(stringify!($name), module_path!()))
					.docs(&["Commitment to polynomial"])
					.composite(scale_info::build::Fields::named().field(|f| {
						f.ty::<[u8; $size]>().name(stringify!(inner)).type_name("G1Affine")
					}))
			}
		}
	};
}

kzg_data_size!(KZGCommitment, FsG1, 48);
kzg_data_size!(KZGProof, FsG1, 48);
kzg_data_size!(BlsScalar, FsFr, 32);

pub const BYTES_PER_BLOB: usize = BYTES_PER_FIELD_ELEMENT * FIELD_ELEMENTS_PER_BLOB;
pub const BYTES_PER_FIELD_ELEMENT: usize = 32;
pub const FIELD_ELEMENTS_PER_BLOB: usize = 4;

#[derive(Debug, Default, Clone, PartialEq, Eq, From, AsRef, AsMut, Deref, DerefMut)]
#[repr(transparent)]
pub struct Blob(pub Vec<FsFr>);

impl Blob {
	#[inline]
	pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, String> {
		if bytes.len() > BYTES_PER_BLOB {
			return Err(format!(
				"Invalid byte length. Expected maximum {} got {}",
				BYTES_PER_BLOB,
				bytes.len(),
			));
		}
		let data_result: Result<Vec<FsFr>, String> = bytes
			.chunks(BYTES_PER_FIELD_ELEMENT)
			.map(|chunk| {
				chunk
					.try_into()
					.map_err(|_| "Chunked into incorrect number of bytes".to_string())
					.and_then(FsFr::from_bytes)
			})
			.collect();

		data_result.map(|mut data| {
			if data.len() == FIELD_ELEMENTS_PER_BLOB {
				Self(data)
			} else {
				data.resize(FIELD_ELEMENTS_PER_BLOB, FsFr::zero());
				Self(data)
			}
		})
	}

	#[inline]
	pub fn vec_to_repr(value: Vec<Self>) -> Vec<Vec<FsFr>> {
		unsafe {
			let mut value = mem::ManuallyDrop::new(value);
			Vec::from_raw_parts(value.as_mut_ptr() as *mut Vec<FsFr>, value.len(), value.capacity())
		}
	}

	#[inline]
	pub fn slice_to_repr(value: &[Self]) -> &[Vec<FsFr>] {
		unsafe { mem::transmute(value) }
	}
}
