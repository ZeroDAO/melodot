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

#![cfg(test)]

use super::*;
use crate as pallet_melo_store;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
};
use sp_runtime::testing::UintAuthorityId;

// Utility function to report unavailability
pub fn report_unavailability(
	who: u32,
	at_block: u64,
	index_set: Vec<u32>,
	validators_len: u32,
) -> DispatchResult {
	let report = pallet_melo_store::UnavailableDataReport {
		at_block,
		authority_index: who,
		index_set,
		validators_len,
	};
	let signature = UintAuthorityId((who + 1).into()).sign(&report.encode()).unwrap();

	MeloStore::pre_dispatch(&crate::Call::report {
		unavailable_data_report: report.clone(),
		signature: signature.clone(),
	})
	.map_err(|e| match e {
		TransactionValidityError::Invalid(InvalidTransaction::Custom(INVALID_VALIDATORS_LEN)) =>
			"invalid validators len",
		e @ _ => <&'static str>::from(e),
	})?;

	MeloStore::report(RuntimeOrigin::none(), report, signature)
}

// Utility function to report unavailability
pub fn submit_init_data() -> DispatchResult {
	MeloStore::register_app(RuntimeOrigin::signed(1))?;
	let app_id = 1;
	let bytes_len = 10;
	let (commitments, proofs) = commits_and_proofs(bytes_len, 0);

	submit_data(2, app_id, bytes_len, 1u32, commitments, proofs)
}

// Utility function to submit data
pub fn submit_data(
	who: u64,
	app_id: u32,
	bytes_len: u32,
	nonce: u32,
	commitments: Vec<KZGCommitment>,
	proofs: Vec<KZGProof>,
) -> DispatchResult {
	MeloStore::register_app(RuntimeOrigin::signed(1))?;
	MeloStore::submit_data(
		RuntimeOrigin::signed(who),
		SidecarMetadata::new(app_id, bytes_len, nonce, commitments, proofs),
	)
}

fn commits_and_proofs(bytes_len: u32, reduction: usize) -> (Vec<KZGCommitment>, Vec<KZGProof>) {
	let len = Blob::blob_count(bytes_len as usize, BYTES_PER_BLOB);
	let adjusted_len = if len > reduction { len - reduction } else { 0 };

	let commits: Vec<KZGCommitment> = (0..adjusted_len).map(|_| KZGCommitment::rand()).collect();
	let proofs: Vec<KZGProof> = (0..adjusted_len).map(|_| KZGProof::rand()).collect();

	(commits, proofs)
}

fn events() -> Vec<Event<Runtime>> {
	let result = System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(
			|e| if let mock::RuntimeEvent::MeloStore(inner) = e { Some(inner) } else { None },
		)
		.collect::<Vec<_>>();

	System::reset_events();

	result
}

fn set_keys() {
	advance_session();
	advance_session();

	assert_eq!(Session::validators(), vec![1, 2, 3]);
}

#[test]
fn should_submit_data_successfully() {
	new_test_ext().execute_with(|| {
		let app_id = 1;
		let bytes_len = 100_000;
		let (commitments, proofs) = commits_and_proofs(bytes_len, 0);

		assert_ok!(submit_data(1, app_id, bytes_len, 1u32, commitments.clone(), proofs.clone()));
		let block_number = System::block_number();
		let metadata = Metadata::<Runtime>::get(block_number);
		assert_eq!(metadata.len(), 1);
		assert_eq!(metadata[0].app_id, app_id);
		assert_eq!(metadata[0].bytes_len, bytes_len);
		assert_eq!(metadata[0].commitments, commitments);
	});
}

#[test]
fn should_fail_when_submitting_data_exceeds_limit() {
	new_test_ext().execute_with(|| {
		let app_id = 1;
		let bytes_len = MAX_BLOB_NUM * (BYTES_PER_BLOB as u32) + 1; // Exceeding the limit
		let (commitments, proofs) = commits_and_proofs(bytes_len, 0);

		assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(1)));

		assert_noop!(
			MeloStore::submit_data(
				RuntimeOrigin::signed(2),
				SidecarMetadata::new(app_id, bytes_len, 1, commitments, proofs),
			),
			Error::<Runtime>::ExceedMaxBlobLimit
		);
	});
}

#[test]
fn should_fail_when_submitting_invalid_app_id() {
	new_test_ext().execute_with(|| {
		let app_id = 9999; // Invalid app_id
		let bytes_len = 10;
		let (commitments, proofs) = commits_and_proofs(bytes_len, 0);

		assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(1)));

		assert_noop!(
			MeloStore::submit_data(
				RuntimeOrigin::signed(2),
				SidecarMetadata::new(app_id, bytes_len, 1u32, commitments.clone(), proofs.clone()),
			),
			Error::<Runtime>::AppIdError
		);
	});
}

#[test]
fn should_emit_event_on_successful_submission() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let who = 1;
		let app_id = 1;
		let bytes_len = 10;
		let (commitments, proofs) = commits_and_proofs(bytes_len, 0);
		let nonce = 1;

		assert_ok!(submit_data(who, app_id, bytes_len, nonce, commitments.clone(), proofs.clone()));

		assert!(events().contains(&Event::<Runtime>::DataReceived {
			from: who,
			app_id,
			index: 0,
			commitments,
			proofs,
			bytes_len,
		}));
	});
}

#[test]
fn should_report_unavailable_data_successfully() {
	new_test_ext().execute_with(|| {
		set_keys();

		let now = System::block_number();

		// Submit data
		assert_ok!(submit_init_data());

		System::set_block_number(((now as u32) + DELAY_CHECK_THRESHOLD).into());

		assert_noop!(report_unavailability(100, now, vec![0], 3,), "Transaction is outdated");

		let authority_index = 1;

		assert_noop!(
			report_unavailability(authority_index, now, vec![0], 100,),
			"invalid validators len"
		);

		assert_ok!(report_unavailability(authority_index, now, vec![0], 3,));

		if let Some(vote) = UnavailableVote::<Runtime>::get(now, 0) {
			assert!(vote.contains(&authority_index));
			assert!(vote.len() == 1);
		} else {
			assert!(false);
		}

		assert!(events().contains(&Event::<Runtime>::ReportReceived { at_block: now, from: 1 }));

		let metadata = Metadata::<Runtime>::get(now);
		assert_eq!(metadata[0].is_available, true);

		let authority_index = 2;

		// Report unavailability again
		assert_ok!(report_unavailability(authority_index, now, vec![0], 3,));

		if let Some(vote) = UnavailableVote::<Runtime>::get(now, 0) {
			assert!(vote.contains(&authority_index));
			assert!(vote.len() == 2);
		} else {
			assert!(false);
		}

		assert!(events().contains(&Event::<Runtime>::ReportReceived { at_block: now, from: 2 }));

		// Check if the metadata's availability status has changed
		let metadata = Metadata::<Runtime>::get(now);
		assert_eq!(metadata[0].is_available, false);
	});
}

#[test]
fn should_report_unavailable_data_successfully_with_multiple_app_id_and_data() {
	new_test_ext().execute_with(|| {
		set_keys();
		let now = System::block_number();

		for app_id in 1..=10u32 {
			assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(app_id as u64)));

			for _ in 1..=10 {
				let bytes_len = 10;
				let (commitments, proofs) = commits_and_proofs(bytes_len, 0);

				assert_ok!(MeloStore::submit_data(
					RuntimeOrigin::signed(app_id as u64),
					SidecarMetadata::new(
						app_id,
						bytes_len,
						1,
						commitments.clone(),
						proofs.clone()
					)
				));
			}
		}

		System::set_block_number((now + (DELAY_CHECK_THRESHOLD as u64)).into());

		let mut index_set = vec![];
		for index in 0..10 * 10 {
			index_set.push(index);
		}

		assert_ok!(report_unavailability(1, now, vec![0], 3,));

		assert!(events().contains(&Event::<Runtime>::ReportReceived { at_block: now, from: 1 }));
	});
}

#[test]
fn should_fail_when_reporting_outside_window() {
	new_test_ext().execute_with(|| {
		set_keys();
		let now = System::block_number();
		System::set_block_number((10 + DELAY_CHECK_THRESHOLD + 1).into()); // Outside the window

		assert_noop!(
			report_unavailability(1, now, vec![0, 1], 3,),
			Error::<Runtime>::ExceedUnavailableDataConfirmTime
		);
	});
}

#[test]
fn should_fail_when_index_set_empty() {
	new_test_ext().execute_with(|| {
		set_keys();
		let now = System::block_number();

		// Submit data
		assert_ok!(submit_init_data());

		assert_noop!(report_unavailability(1, now, vec![], 3,), Error::<Runtime>::IndexSetIsEmpty);
	});
}

#[test]
fn should_fail_when_reporting_duplicate_indices() {
	new_test_ext().execute_with(|| {
		set_keys();
		let now = System::block_number();

		// Submit data
		assert_ok!(submit_init_data());

		assert_noop!(
			report_unavailability(1, now, vec![0, 0], 3,),
			Error::<Runtime>::DuplicateReportSubmission
		);
	});
}

#[test]
fn should_fail_when_reporting_nonexistent_data() {
	new_test_ext().execute_with(|| {
		set_keys();
		let now = System::block_number();

		// Submit data
		assert_ok!(submit_init_data());

		System::set_block_number((now + (DELAY_CHECK_THRESHOLD as u64)).into());

		assert_noop!(
			report_unavailability(1, now, vec![99999], 3,),
			Error::<Runtime>::DataNotExist
		);
	});
}

#[test]
fn should_increment_app_id_on_consecutive_registrations() {
	new_test_ext().execute_with(|| {
		assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(1)));
		assert_eq!(AppId::<Runtime>::get(), 1);

		assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(2)));
		assert_eq!(AppId::<Runtime>::get(), 2);
	});
}

#[test]
fn should_emit_event_on_successful_registration() {
	new_test_ext().execute_with(|| {
		let who = 10;
		System::set_block_number(10);

		assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(who)));
		assert!(events().contains(&Event::<Runtime>::AppIdRegistered { app_id: 1, from: who }));

		assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(who)));
		assert!(events().contains(&Event::<Runtime>::AppIdRegistered { app_id: 2, from: who }));

		assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(122)));
		assert!(events().contains(&Event::<Runtime>::AppIdRegistered { app_id: 3, from: 122 }));
	});
}

#[test]
fn should_fail_when_reporting_for_future_block() {
	new_test_ext().execute_with(|| {
		set_keys();
		let now = System::block_number();

		// Submit data
		assert_ok!(submit_init_data());

		// Try to report unavailability for a future block (e.g., now + 5)
		assert_noop!(
			report_unavailability(1, now + 5, vec![0], 3),
			Error::<Runtime>::ReportForFutureBlock
		);
	});
}

#[test]
fn should_fail_when_submitting_empty_data() {
	new_test_ext().execute_with(|| {
		let app_id = 1;
		let bytes_len = 0; // Setting the data length to 0 to trigger the error.
		let (commitments, proofs) = commits_and_proofs(bytes_len, 0);

		assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(1)));

		assert_noop!(
			MeloStore::submit_data(
				RuntimeOrigin::signed(2),
				SidecarMetadata::new(app_id, bytes_len, 1, commitments.clone(), proofs.clone()),
			),
			Error::<Runtime>::SubmittedDataIsEmpty
		);
	});
}

#[test]
fn should_fail_with_mismatched_commitments_count() {
	new_test_ext().execute_with(|| {
		let app_id = 1;
		let bytes_len = 10;
		let (commitments, proofs) = commits_and_proofs(bytes_len, 1);

		assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(1)));

		assert_noop!(
			MeloStore::submit_data(
				RuntimeOrigin::signed(2),
				SidecarMetadata::new(app_id, bytes_len, 1, commitments.clone(), proofs.clone()),
			),
			Error::<Runtime>::MismatchedCommitmentsCount
		);
	});
}

#[test]
fn should_fail_with_mismatched_proofs_count() {
	new_test_ext().execute_with(|| {
		let app_id = 1;
		let bytes_len = 10;
		let (commitments, proofs) = commits_and_proofs(bytes_len, 1);

		let mut commitments = commitments;
		commitments.push(KZGCommitment::rand());

		assert_ok!(MeloStore::register_app(RuntimeOrigin::signed(1)));

		assert_noop!(
			MeloStore::submit_data(
				RuntimeOrigin::signed(2),
				SidecarMetadata::new(app_id, bytes_len, 1, commitments.clone(), proofs.clone()),
			),
			Error::<Runtime>::MismatchedProofsCount
		);
	});
}

#[test]
fn should_change_metadata_availability_when_reports_exceed_threshold() {
	new_test_ext().execute_with(|| {
		set_keys();

		let now = System::block_number();

		// Submit data
		assert_ok!(submit_init_data());

		// Set a threshold for this test
		let threshold = 2;

		// Report unavailability multiple times to exceed the threshold
		for i in 0..threshold {
			assert_ok!(report_unavailability(i, now, vec![0], 3));
		}

		// Check if the metadata's availability status has changed
		let metadata = Metadata::<Runtime>::get(now);
		assert_eq!(metadata[0].is_available, false);
	});
}

#[test]
fn should_have_expected_data_when_reported_unavailable() {
	new_test_ext().execute_with(|| {
		set_keys();

		let now = System::block_number();

		// Submit data
		let (commitments, proofs) = commits_and_proofs(10, 0);
		assert_ok!(submit_data(1, 1, 10, 1, commitments, proofs));

		// Report unavailability
		assert_ok!(report_unavailability(1, now, vec![0], 3));

		// Check if the reported data matches the expected data
		let metadata = Metadata::<Runtime>::get(now);
		assert_eq!(metadata[0].is_available, true);
	});
}

#[test]
fn should_send_single_unavailability_report_correctly() {
	let mut ext = new_test_ext();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		let now = 10;
		System::set_block_number(now);

		// Submit data
		assert_ok!(submit_init_data());

		// Call the send_single_unavailability_report function
		let authority_index = 1;
		let key = UintAuthorityId(authority_index.into());
		let at_block = now - 1;
		let index_set = vec![0];
		let result = MeloStore::send_single_unavailability_report(
			authority_index,
			key,
			at_block.into(),
			now.into(),
			index_set,
		);
		assert!(result.is_ok());
	});
}

#[test]
fn should_acquire_and_release_report_lock_correctly() {
	let mut ext = new_test_ext();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		let now = 10;
		System::set_block_number(now);

		// Call the with_report_lock function
		let authority_index = 1;
		let at_block = now - 1;
		let result =
			MeloStore::with_report_lock(authority_index, at_block.into(), now.into(), || Ok(()));
		assert!(result.is_ok());

		// Try to acquire the lock again, it should fail
		let failed_result =
			MeloStore::with_report_lock(authority_index, at_block.into(), now.into(), || Ok(()));
		assert!(failed_result.is_err());
	});
}

// #[test]
// fn should_send_unavailability_report_correctly() {
// 	let mut ext = new_test_ext();
// 	let (offchain, _state) = TestOffchainExt::new();
// 	let (pool, _) = TestTransactionPoolExt::new();
// 	ext.register_extension(OffchainDbExt::new(offchain.clone()));
// 	ext.register_extension(OffchainWorkerExt::new(offchain));
// 	ext.register_extension(TransactionPoolExt::new(pool));

// 	ext.execute_with(|| {
// 		let now = 10;
// 		System::set_block_number(now);

// 		assert!(MeloStore::register_app(RuntimeOrigin::signed(1)).is_ok());
// 		let app_id = 1;
// 		let bytes_len = 121; // Exceeding the limit
// 		let data_hash = H256::random();
// 		let (commitments, proofs) = commits_and_proofs(bytes_len, 0);

// 		assert_ok!(MeloStore::submit_data(
// 			RuntimeOrigin::signed(2),
// 			app_id,
// 			bytes_len,
// 			0u32,
// 			commitments.clone(),
// 			proofs.clone(),
// 		));
// 		let sidecar_metadata =
// 			SidecarMetadata { data_len: bytes_len, blobs_hash: data_hash, commitments, proofs };

// 		let mut sidecar = Sidecar::new(sidecar_metadata, None);
// 		sidecar.set_not_found();
// 		sidecar.save_to_local();
// 		assert!(sidecar.is_unavailability());

// 		// Test get_unavailability_data
// 		let unavailability_data = MeloStore::get_unavailability_data(now);
// 		assert!(unavailability_data.contains(&0));

// 		assert!(MeloStore::send_unavailability_report(now).ok().is_some());

// 		let now = now + (DELAY_CHECK_THRESHOLD as u64) + 10;
// 		System::set_block_number(now);
// 		let mut res = MeloStore::send_unavailability_report(now).unwrap();
// 		assert!(res.next().is_none());
// 	});
// }
