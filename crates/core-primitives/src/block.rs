// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::traits::{
	ExtendedBlock as ExtendedBlockT, ExtendedHeader as ExtendedHeaderT, ExtendedHeaderProvider,
};
use codec::Codec;

use sp_runtime::{
	generic::Block,
	traits::{Extrinsic as ExtrinsicT, MaybeSerialize, MaybeSerializeDeserialize, Member},
};

// #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
// #[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub type ExtendedBlock<ExtendedHeader, Extrinsic> = Block<ExtendedHeader, Extrinsic>;

impl<ExtendedHeader, Extrinsic, Number> ExtendedBlockT<Number>
	for ExtendedBlock<ExtendedHeader, Extrinsic>
where
	ExtendedHeader: ExtendedHeaderT<Number> + MaybeSerializeDeserialize,
	Extrinsic: Member + Codec + ExtrinsicT + MaybeSerialize,
{
	type ExtendedHeader = ExtendedHeader;
}

impl<Header, Extrinsic, Number> ExtendedHeaderProvider<Number> for ExtendedBlock<Header, Extrinsic>
where
	Header: ExtendedHeaderT<Number>,
{
	type ExtendedHeaderT = Header;
}
