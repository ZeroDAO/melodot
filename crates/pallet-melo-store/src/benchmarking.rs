// Copyright 2023 ZeroDAO

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as MeloStore;
use frame_benchmarking::v1::{benchmarks, whitelisted_caller};
use frame_support::{traits::UnfilteredDispatchable, WeakBoundedVec};
use frame_system::RawOrigin;
use sp_core::H256;
use sp_runtime::{
	traits::{ValidateUnsigned, Zero},
	transaction_validity::TransactionSource,
};

const MAX_KEYS: u32 = 1000;
const MAX_BLOB_INDEX: u32 = 100;

pub fn create_report<T: Config>(
	k: u32,
	e: u32,
) -> Result<
	(
		crate::UnavailableDataReport<frame_system::pallet_prelude::BlockNumberFor<T>>,
		<T::AuthorityId as RuntimeAppPublic>::Signature,
	),
	&'static str,
> {
	let mut keys = Vec::new();
	for _ in 0..k {
		keys.push(T::AuthorityId::generate_pair(None));
	}
	let bounded_keys = WeakBoundedVec::<_, T::MaxKeys>::try_from(keys.clone())
		.map_err(|()| "More than the maximum number of keys provided")?;
	Keys::<T>::put(bounded_keys);

	// 1. Set a valid app_id
	AppId::<T>::put(1u32);

	// 2. Create and insert BlobMetadata
	let caller: T::AccountId = whitelisted_caller();
	let app_id = 1u32;

	let commitments = vec![Default::default()];
	let proofs = vec![Default::default()];

	let current_block_number = <frame_system::Pallet<T>>::block_number();

	let at_block = current_block_number;

	let mut metadata_list = Vec::new();
	for _ in 0..MAX_BLOB_INDEX {
		let blob_metadata = BlobMetadata::<T> {
			app_id,
			from: caller.clone(),
			commitments: BoundedVec::<_, T::MaxBlobNum>::try_from(commitments.clone())
				.map_err(|_| "commitments Vec too large")?,
			proofs: BoundedVec::<_, T::MaxBlobNum>::try_from(proofs.clone())
				.map_err(|_| "proofs Vec too large")?,
			bytes_len: 10u32,
			data_hash: H256::default(),
			is_available: true,
		};

		metadata_list.push(blob_metadata.clone());
	}

	let bounded_metadata = WeakBoundedVec::<_, T::MaxKeys>::try_from(metadata_list.clone())
		.map_err(|_| "Metadata Vec too large")?;
	Metadata::<T>::insert(at_block, bounded_metadata);

	let mut index_set = Vec::new();
	for i in 0..e {
		index_set.push(i);
	}

	let input_report =
		UnavailableDataReport { at_block, authority_index: k - 1, index_set, validators_len: k };

	let encoded_report = input_report.encode();
	let authority_id = keys.get((k - 1) as usize).ok_or("out of range")?;
	let signature = authority_id.sign(&encoded_report).ok_or("couldn't make signature")?;
	Ok((input_report, signature))
}

benchmarks! {
	register_app {
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller.clone()))
	verify {
		ensure!(AppId::<T>::get() == 1, "AppId should be incremented to 1");
	}

	submit_data {
		let k in 1 .. T::MaxBlobNum::get();

		let caller: T::AccountId = whitelisted_caller();
		AppId::<T>::put(1u32);

		let commitments: Vec<KZGCommitment> = vec![Default::default(); k as usize];
		let proofs: Vec<KZGProof> = vec![Default::default(); k as usize];

		let bytes_len = (k * BYTES_PER_BLOB as u32) as u32;
		let data_hash = H256::default();
		let app_id = 1u32;

	}: _(RawOrigin::Signed(caller.clone()), app_id, bytes_len, data_hash, commitments.clone(), proofs.clone())
	verify {
		let current_block_number = <frame_system::Pallet<T>>::block_number();
		let metadata_list = Metadata::<T>::get(&current_block_number);
		ensure!(metadata_list.len() == 1, "Expected one metadata item.");
		let metadata = &metadata_list[0];
		ensure!(metadata.app_id == app_id, "Unexpected app_id in metadata.");
		ensure!(metadata.from == caller, "Unexpected caller in metadata.");
		ensure!(metadata.commitments == commitments, "Unexpected commitments in metadata.");
		ensure!(metadata.proofs == proofs, "Unexpected proofs in metadata.");
		ensure!(metadata.bytes_len == bytes_len, "Unexpected bytes_len in metadata.");
		ensure!(metadata.data_hash == data_hash, "Unexpected data_hash in metadata.");
	}

	#[extra]
	report {
		let k in 1 .. MAX_KEYS;
		let e in 1 .. MAX_BLOB_INDEX;
		let (input_report, signature) = create_report::<T>(k,e)?;
	}: _(RawOrigin::None, input_report, signature)

	#[extra]
	validate_unsigned {
		let k in 1 .. MAX_KEYS;
		let e in 1 .. MAX_BLOB_INDEX;
		let (input_report, signature) = create_report::<T>(k,e)?;
		let call = Call::report { unavailable_data_report: input_report, signature };
	}: {
		MeloStore::<T>::validate_unsigned(TransactionSource::InBlock, &call)
			.map_err(<&str>::from)?;
	}

	validate_unsigned_and_then_report {
		let k in 1 .. MAX_KEYS;
		let e in 1 .. MAX_BLOB_INDEX;
		let (input_report, signature) = create_report::<T>(k,e)?;
		let call = Call::report { unavailable_data_report: input_report, signature };
		let call_enc = call.encode();
	}: {
		MeloStore::<T>::validate_unsigned(TransactionSource::InBlock, &call).map_err(<&str>::from)?;
		<Call<T> as Decode>::decode(&mut &*call_enc)
			.expect("call is encoded above, encoding must be correct")
			.dispatch_bypass_filter(RawOrigin::None.into())?;
	}

	impl_benchmark_test_suite!(MeloStore, crate::mock::new_test_ext(), crate::mock::Runtime);
}
