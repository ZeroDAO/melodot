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

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod mock;
mod tests;

pub mod weights;
pub use weights::*;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	pallet_prelude::*,
	traits::{Get, OneSessionHandler},
	BoundedSlice, WeakBoundedVec,
};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use melo_das_primitives::{blob::Blob, config::BYTES_PER_BLOB};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::{
	offchain::storage::{MutateStorageError, StorageRetrievalError, StorageValueRef},
	traits::{AtLeast32BitUnsigned, Saturating},
	Permill, RuntimeDebug,
};
use sp_std::prelude::*;

use melo_core_primitives::{
	confidence::{ConfidenceId, ConfidenceManager},
	config::{BLOCK_SAMPLE_LIMIT, MAX_UNAVAILABLE_BLOCK_INTERVAL},
	extension::AppLookup,
	traits::HeaderCommitList,
	SidecarMetadata,
};
use melo_das_db::offchain::OffchainKv;
use melo_das_primitives::crypto::{KZGCommitment, KZGProof};

// A prefix constant used for the off-chain database.
const DB_PREFIX: &[u8] = b"melodot/melo-store/unavailable-data-report";
// A threshold constant used to determine when to delay the acknowledgment of unavailability.
pub const DELAY_CHECK_THRESHOLD: u32 = 1;
// Weight constant for each blob.
pub const WEIGHT_PER_BLOB: Weight = Weight::from_parts(1024, 0);

// Typedef for Authorization Index.
pub type AuthIndex = u32;

// Struct to represent the report status.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
struct ReportStatus<BlockNumber> {
	/// The block number when the data was reported.
	pub at_block: BlockNumber,
	/// The block number when the report was sent.
	pub sent_at: BlockNumber,
}

impl<BlockNumber: PartialEq + AtLeast32BitUnsigned + Copy> ReportStatus<BlockNumber> {
	// Check if the report is recent based on given parameters.
	fn is_recent(&self, at_block: BlockNumber, now: BlockNumber) -> bool {
		self.at_block == at_block && self.sent_at + DELAY_CHECK_THRESHOLD.into() > now
	}
}

/// Possible errors that can occur during off-chain execution.
#[cfg_attr(test, derive(PartialEq))]
enum OffchainErr<BlockNumber> {
	WaitingForInclusion(BlockNumber),
	FailedSigning,
	FailedToAcquireLock,
	SubmitTransaction,
}

impl<BlockNumber: sp_std::fmt::Debug> sp_std::fmt::Debug for OffchainErr<BlockNumber> {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::WaitingForInclusion(ref block) => {
				write!(fmt, "Report already sent at {:?}. Waiting for inclusion.", block)
			},
			OffchainErr::FailedSigning => write!(fmt, "Failed to sign report"),
			OffchainErr::FailedToAcquireLock => write!(fmt, "Failed to acquire lock"),
			OffchainErr::SubmitTransaction => write!(fmt, "Failed to submit transaction"),
		}
	}
}

// Typedef for results returned by off-chain operations.
type OffchainResult<T, A> = Result<A, OffchainErr<BlockNumberFor<T>>>;

// Struct to represent a report of unavailable data.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct UnavailableDataReport<BlockNumber>
where
	BlockNumber: PartialEq + Eq + Decode + Encode,
{
	/// Block number at the time report is created.
	pub at_block: BlockNumber,
	/// Index of the authority reporting the unavailability.
	pub authority_index: AuthIndex,
	/// Set of indexes related to the report.
	pub index_set: Vec<u32>,
	/// Total length of session validator set.
	pub validators_len: u32,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	pub type KZGCommitmentListFor<T> = BoundedVec<KZGCommitment, <T as Config>::MaxBlobNum>;
	pub type KZGProofListFor<T> = BoundedVec<KZGProof, <T as Config>::MaxBlobNum>;

	/// Represents the metadata of a blob in the system.
	#[derive(Clone, Eq, Default, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct BlobMetadata<T: Config> {
		/// Unique identifier for the application that uses this blob.
		pub app_id: u32,

		/// Account ID of the entity that created or owns this blob.
		pub from: T::AccountId,

		/// List of KZG commitments associated with this blob.
		pub commitments: KZGCommitmentListFor<T>,

		/// List of KZG proofs associated with this blob.
		pub proofs: KZGProofListFor<T>,

		/// Length of the data in bytes that this metadata represents.
		pub bytes_len: u32,

		/// Flag indicating whether the blob data is available or not.
		pub is_available: bool,

		pub nonce: u32,
	}

	/// Provides configuration parameters for the pallet.
	#[pallet::config]
	pub trait Config: SendTransactionTypes<Call<Self>> + frame_system::Config {
		/// This type represents an event in the runtime, which includes events emitted by this
		/// pallet.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// This type represents the computation cost of the pallet's operations.
		type WeightInfo: WeightInfo;

		/// This type defines the unique identifier for an authority or a trusted node in the
		/// network.
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// Defines the upper limit for the number of keys that can be stored.
		type MaxKeys: Get<u32>;

		/// The maximum number of blobs that can be handled.
		#[pallet::constant]
		type MaxBlobNum: Get<u32>;

		/// This defines the priority for unsigned transactions in the Melo context.
		#[pallet::constant]
		type MeloUnsignedPriority: Get<TransactionPriority>;
	}

	/// Represents metadata associated with the AppData. It's preserved for future verification.
	/// Deleting data after a certain point may be beneficial for storage and computational
	/// efficiency.
	#[pallet::storage]
	#[pallet::getter(fn metadata)]
	pub(super) type Metadata<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		WeakBoundedVec<BlobMetadata<T>, T::MaxKeys>,
		ValueQuery,
	>;

	/// Contains the keys for this pallet's use.
	#[pallet::storage]
	#[pallet::getter(fn keys)]
	pub(super) type Keys<T: Config> =
		StorageValue<_, WeakBoundedVec<T::AuthorityId, T::MaxKeys>, ValueQuery>;

	/// Holds the unique identifier for the application using this pallet.
	#[pallet::storage]
	#[pallet::getter(fn app_id)]
	pub(super) type AppId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	pub(super) type Nonces<T: Config> = StorageMap<_, Twox64Concat, u32, u32, ValueQuery>;

	/// Represents votes regarding the availability of certain data.
	#[pallet::storage]
	#[pallet::getter(fn unavailable_vote)]
	pub(super) type UnavailableVote<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		Twox64Concat,
		u32,
		WeakBoundedVec<AuthIndex, T::MaxBlobNum>,
	>;

	/// Enumerates all the possible events that can be emitted by this pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Indicates that data was successfully received.
		DataReceived {
			bytes_len: u32,
			from: T::AccountId,
			app_id: u32,
			index: u32,
			commitments: Vec<KZGCommitment>,
			proofs: Vec<KZGProof>,
		},
		/// Signifies that a report has been submitted.
		ReportReceived { at_block: BlockNumberFor<T>, from: AuthIndex },
		/// Denotes the successful registration of a new application ID.
		AppIdRegistered { app_id: u32, from: T::AccountId },
	}

	/// Enumerates all possible errors that might occur while using this pallet.
	#[pallet::error]
	pub enum Error<T> {
		/// The system has reached its limit for the number of Blobs.
		ExceedMaxBlobLimit,
		/// Something's wrong with the given app ID.
		AppIdError,
		/// Too many Blobs were added in a single block.
		ExceedMaxBlobPerBlock,
		/// The time for confirming unavailable data has passed.
		ExceedUnavailableDataConfirmTime,
		/// No indices have been set.
		IndexSetIsEmpty,
		/// The requested data doesn't exist.
		DataNotExist,
		/// The same report was submitted more than once.
		DuplicateReportSubmission,
		/// The total votes have exceeded the allowed maximum.
		ExceedMaxTotalVotes,
		/// A report was made for a block that hasn't occurred yet.
		ReportForFutureBlock,
		/// No data was provided in the submission.
		SubmittedDataIsEmpty,
		/// The number of provided commitments doesn't match the expected number.
		MismatchedCommitmentsCount,
		/// The number of provided proofs doesn't match the expected number.
		MismatchedProofsCount,
		/// The provided public key is not valid.
		InvalidKey,
		/// The nonce is invalid.
		NonceError,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit data for a particular app.
		/// This call allows a user to submit data, its commitments, and proofs.
		/// The function ensures various constraints like the length of the data, validity of the
		/// app id, and other integrity checks.
		#[pallet::call_index(0)]
		#[pallet::weight(
			WEIGHT_PER_BLOB
			.saturating_mul(params.commitments.len().max(1) as u64)
			.saturating_add(
				<T as Config>::WeightInfo::submit_data(params.proofs.len() as u32)
			)
		)]
		pub fn submit_data(origin: OriginFor<T>, params: SidecarMetadata) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(params.check(), Error::<T>::SubmittedDataIsEmpty);
			// ensure!(params.bytes_len > 0, Error::<T>::SubmittedDataIsEmpty);
			let blob_num = Blob::blob_count(params.bytes_len as usize, BYTES_PER_BLOB);
			ensure!(blob_num <= T::MaxBlobNum::get() as usize, Error::<T>::ExceedMaxBlobLimit);

			// // Check if blob_num matches the length of commitments.
			// ensure!(blob_num == commitments.len(), Error::<T>::MismatchedCommitmentsCount);
			// // Check if blob_num matches the length of proofs.
			// ensure!(blob_num == proofs.len(), Error::<T>::MismatchedProofsCount);

			let current_app_id = AppId::<T>::get();
			ensure!(params.app_id <= current_app_id, Error::<T>::AppIdError);

			// Check if the nonce is valid.
			let current_nonce = Nonces::<T>::get(
				&current_app_id,
			);

			ensure!(params.nonce == current_nonce.saturating_add(1), Error::<T>::NonceError);

			let mut commitment_list: BoundedVec<KZGCommitment, T::MaxBlobNum> =
				BoundedVec::default();
			commitment_list
				.try_extend(params.commitments.iter().cloned())
				.map_err(|_| Error::<T>::ExceedMaxBlobPerBlock)?;

			let mut proof_list: BoundedVec<KZGProof, T::MaxBlobNum> = BoundedVec::default();
			proof_list
				.try_extend(params.proofs.iter().cloned())
				.map_err(|_| Error::<T>::ExceedMaxBlobPerBlock)?;

			let metadata: BlobMetadata<T> = BlobMetadata {
				app_id: params.app_id,
				from: who.clone(),
				commitments: commitment_list,
				bytes_len: params.bytes_len,
				proofs: proof_list,
				// Theoretically, the submitted data is likely to be available,
				// so we initially assume it's available.
				is_available: true,
				nonce: params.nonce,
			};

			let current_block_number = <frame_system::Pallet<T>>::block_number();

			let mut metadata_len = 0;
			Metadata::<T>::try_mutate(current_block_number, |metadata_vec| {
				metadata_len = metadata_vec.len();
				metadata_vec.try_push(metadata).map_err(|_| Error::<T>::ExceedMaxBlobPerBlock)
			})?;

			Nonces::<T>::mutate(
				&current_app_id,
				|nonce| *nonce = params.nonce,
			);

			Self::deposit_event(Event::DataReceived {
				bytes_len: params.bytes_len,
				from: who,
				app_id: params.app_id,
				index: metadata_len as u32,
				commitments: params.commitments,
				proofs: params.proofs,
			});

			Ok(())
		}

		/// Report on the unavailability of certain data.
		/// Validators can use this function to report any data that they find unavailable.
		/// The function does checks like making sure the data isn't being reported for a future
		/// block, the report is within the acceptable delay, and that the reporting key is valid.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::validate_unsigned_and_then_report(
			unavailable_data_report.validators_len,
			unavailable_data_report.index_set.len() as u32,
		))]
		pub fn report(
			origin: OriginFor<T>,
			unavailable_data_report: UnavailableDataReport<BlockNumberFor<T>>,
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;
			let current_block_number: BlockNumberFor<T> = <frame_system::Pallet<T>>::block_number();

			ensure!(
				unavailable_data_report.at_block <= current_block_number,
				Error::<T>::ReportForFutureBlock
			);

			ensure!(
				unavailable_data_report.at_block + DELAY_CHECK_THRESHOLD.into() >=
					current_block_number,
				Error::<T>::ExceedUnavailableDataConfirmTime
			);

			ensure!(!unavailable_data_report.index_set.is_empty(), Error::<T>::IndexSetIsEmpty);

			let keys = Keys::<T>::get();
			ensure!(
				keys.get(unavailable_data_report.authority_index as usize).is_some(),
				Error::<T>::InvalidKey
			);

			let index_set = unavailable_data_report.index_set;

			Metadata::<T>::try_mutate(
				unavailable_data_report.at_block,
				|metadata_vec| -> DispatchResult {
					for &index in &index_set {
						let metadata =
							metadata_vec.get_mut(index as usize).ok_or(Error::<T>::DataNotExist)?;

						Self::handle_vote(
							unavailable_data_report.at_block,
							unavailable_data_report.authority_index,
							index,
							metadata,
						)?;
					}
					Ok(())
				},
			)?;
			Self::deposit_event(Event::ReportReceived {
				at_block: unavailable_data_report.at_block,
				from: unavailable_data_report.authority_index,
			});
			Ok(())
		}

		/// Register a new app with the system.
		/// This function allows a user to register a new app and increments the app ID.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::register_app())]
		pub fn register_app(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let app_id = AppId::<T>::get() + 1;
			AppId::<T>::put(app_id);
			Self::deposit_event(Event::AppIdRegistered { app_id, from: who });
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(now: BlockNumberFor<T>) {
			// Deletion of expired polling data
			if T::BlockNumber::from(DELAY_CHECK_THRESHOLD + 1) >= now {
				return
			}
			let _ = UnavailableVote::<T>::clear_prefix(
				now - (DELAY_CHECK_THRESHOLD + 1).into(),
				T::MaxBlobNum::get(),
				None,
			);
		}

		fn offchain_worker(now: BlockNumberFor<T>) {
			// Only send messages if we are a potential validator.
			if sp_io::offchain::is_validator() {
				for res in Self::send_unavailability_report(now).into_iter().flatten() {
					if let Err(e) = res {
						log::debug!(
							target: "runtime::melo-store",
							"Skipping report at {:?}: {:?}",
							now,
							e,
						)
					}
				}
			// TODO - 报告数据不可用区块
			} else {
				log::trace!(
					target: "runtime::melo-store",
					"Skipping report at {:?}. Not a validator.",
					now,
				)
			}
		}
	}

	pub(crate) const INVALID_VALIDATORS_LEN: u8 = 10;

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::report { unavailable_data_report, signature } = call {
				let keys = Keys::<T>::get();

				let authority_id = match keys.get(unavailable_data_report.authority_index as usize)
				{
					Some(id) => id,
					None => return InvalidTransaction::Stale.into(),
				};

				let keys = Keys::<T>::get();
				if keys.len() as u32 != unavailable_data_report.validators_len {
					return InvalidTransaction::Custom(INVALID_VALIDATORS_LEN).into()
				}

				let signature_valid = unavailable_data_report.using_encoded(|encoded_report| {
					authority_id.verify(&encoded_report, signature)
				});

				if !signature_valid {
					return InvalidTransaction::BadProof.into()
				}

				ValidTransaction::with_tag_prefix("MeloStore")
					.priority(T::MeloUnsignedPriority::get())
					.longevity(DELAY_CHECK_THRESHOLD as u64)
					.propagate(true)
					.build()
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Retrieve the list of indexes representing data unavailability at a given block.
	///
	/// # Arguments
	/// * `at_block` - The block number to check for data unavailability.
	pub fn get_unavailability_apps(at_block: BlockNumberFor<T>) -> Vec<u32> {
		Metadata::<T>::get(at_block)
			.iter()
			.enumerate()
			.filter_map(|(i, metadata)| {
				let mut db = OffchainKv::new(Some(DB_PREFIX));
				match ConfidenceId::app_confidence(metadata.app_id, metadata.nonce)
					.get_confidence(&mut db)
				{
					Some(confidence) =>
						if !confidence.is_availability(80, 95) {
							Some(i as u32)
						} else {
							None
						},
					None => None,
				}
			})
			.collect::<Vec<_>>()
	}

	pub fn fetch_unavailability_blocks() -> Vec<BlockNumberFor<T>> {
		let now = <frame_system::Pallet<T>>::block_number();
		let mut db = OffchainKv::new(Some(DB_PREFIX));

		let last: BlockNumberFor<T> =
			match ConfidenceManager::new(db.clone()).get_last_processed_block() {
				Some(block) => block.into(),
				None => now.saturating_sub(MAX_UNAVAILABLE_BLOCK_INTERVAL.into()),
			};

		let mut unavail_blocks = vec![];

		for i in 0..BLOCK_SAMPLE_LIMIT {
			let process_block = last + i.into();
			if process_block >= now.into() {
				break
			}

			let maybe_avail = {
				let block_hash = <frame_system::Pallet<T>>::block_hash(process_block);
				match ConfidenceId::block_confidence(block_hash.as_ref()).get_confidence(&mut db) {
					Some(confidence) => Some(confidence.is_availability(80, 95)),
					None => None,
				}
			};

			if let Some(avail) = maybe_avail {
				if !avail {
					unavail_blocks.push(process_block)
				}
			} else {
				break
			}
		}
		unavail_blocks
	}

	/// Fetch the list of commitments and app lookups at a given block.
	///
	/// # Arguments
	/// * `at_block` - The block number to fetch commitments from.
	pub fn get_commitments_and_app_lookups(
		at_block: BlockNumberFor<T>,
	) -> (Vec<KZGCommitment>, Vec<AppLookup>) {
		let metadatas = Metadata::<T>::get(at_block);

		let mut app_lookups = Vec::with_capacity(metadatas.len());
		let commitments = metadatas
			.iter()
			.flat_map(|metadata| {
				app_lookups.push(AppLookup {
					app_id: metadata.app_id,
					nonce: metadata.nonce,
					count: metadata.commitments.len() as u16,
				});
				metadata.commitments.iter().cloned()
			})
			.collect::<Vec<_>>();

		(commitments, app_lookups)
	}

	/// Assemble and send unavailability reports for any data that is unavailable.
	///
	/// # Arguments
	/// * `now` - The current block number.
	pub(crate) fn send_unavailability_report(
		now: BlockNumberFor<T>,
	) -> OffchainResult<T, impl Iterator<Item = OffchainResult<T, ()>>> {
		let reports = (0..DELAY_CHECK_THRESHOLD)
			.filter_map(move |gap| {
				if T::BlockNumber::from(gap) > now {
					return None
				}
				let at_block = now - gap.into();
				let index_set = Self::get_unavailability_apps(at_block);
				if !index_set.is_empty() {
					Some(Self::local_authority_keys().flat_map(move |(authority_index, key)| {
						Some(Self::send_single_unavailability_report(
							authority_index,
							key,
							at_block,
							now,
							index_set.clone(),
						))
					}))
				} else {
					None
				}
			})
			.flatten();

		Ok(reports)
	}

	// Helper method to send a single unavailability report.
	fn send_single_unavailability_report(
		authority_index: u32,
		key: T::AuthorityId,
		at_block: BlockNumberFor<T>,
		now: BlockNumberFor<T>,
		index_set: Vec<u32>,
	) -> OffchainResult<T, ()> {
		let prepare_unavailable_data_report = || -> OffchainResult<T, Call<T>> {
			let validators_len = Keys::<T>::decode_len().unwrap_or_default() as u32;
			let unavailable_data_report =
				UnavailableDataReport { at_block, authority_index, index_set, validators_len };

			let signature =
				key.sign(&unavailable_data_report.encode()).ok_or(OffchainErr::FailedSigning)?;

			Ok(Call::report { unavailable_data_report, signature })
		};

		Self::with_report_lock(authority_index, at_block, now, || {
			let call = prepare_unavailable_data_report()?;
			log::info!(
				target: "runtime::melo-store",
				"[index: {:?}] Reporting unavailable data of {:?} (at block: {:?}) : {:?}",
				authority_index,
				at_block,
				now,
				call,
			);

			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
				.map_err(|_| OffchainErr::SubmitTransaction)?;

			Ok(())
		})
	}

	// Locking mechanism to prevent double reporting by the same authority.
	fn with_report_lock<R>(
		authority_index: u32,
		at_block: BlockNumberFor<T>,
		now: BlockNumberFor<T>,
		f: impl FnOnce() -> OffchainResult<T, R>,
	) -> OffchainResult<T, R> {
		let mut key = DB_PREFIX.to_vec();
		key.extend(authority_index.encode());

		let storage = StorageValueRef::persistent(&key);

		match storage.mutate(
			|status: Result<Option<ReportStatus<BlockNumberFor<T>>>, StorageRetrievalError>| {
				if let Ok(Some(status)) = status {
					if status.is_recent(at_block, now) {
						return Err(OffchainErr::WaitingForInclusion(status.sent_at))
					}
				}
				Ok(ReportStatus { at_block, sent_at: now })
			},
		) {
			Err(MutateStorageError::ValueFunctionFailed(err)) => Err(err),
			res => {
				let mut new_status = res.map_err(|_| OffchainErr::FailedToAcquireLock)?;
				let result = f();
				if result.is_err() {
					new_status.sent_at = 0u32.into();
					storage.set(&new_status);
				}
				result
			},
		}
	}

	// Fetch all local authority keys.
	fn local_authority_keys() -> impl Iterator<Item = (u32, T::AuthorityId)> {
		let authorities = Keys::<T>::get();

		let mut local_keys = T::AuthorityId::all();

		local_keys.sort();

		authorities.into_iter().enumerate().filter_map(move |(index, authority)| {
			local_keys
				.binary_search(&authority)
				.ok()
				.map(|location| (index as u32, local_keys[location].clone()))
		})
	}

	// Handle an unavailability vote for a particular piece of data.
	fn handle_vote(
		at_block: BlockNumberFor<T>,
		authority_index: AuthIndex,
		metadata_index: u32,
		metadata: &mut BlobMetadata<T>,
	) -> Result<(), DispatchError> {
		const UNAVAILABILITY_THRESHOLD: Permill = Permill::from_percent(50);

		UnavailableVote::<T>::try_mutate(at_block, metadata_index, |maybe_unavailable_vote| {
			let vote_num = match maybe_unavailable_vote {
				Some(unavailable_vote) => {
					// Repeat submission or not
					ensure!(
						!unavailable_vote.contains(&authority_index),
						Error::<T>::DuplicateReportSubmission
					);

					unavailable_vote
						.try_push(authority_index)
						.map_err(|_| Error::<T>::ExceedMaxTotalVotes)?;

					unavailable_vote.len()
				},
				None => {
					let mut new_unavailable_vote =
						WeakBoundedVec::<AuthIndex, T::MaxBlobNum>::default();
					new_unavailable_vote
						.try_push(authority_index)
						.map_err(|_| Error::<T>::ExceedMaxTotalVotes)?;

					*maybe_unavailable_vote = Some(new_unavailable_vote);
					1
				},
			};

			let threshold = UNAVAILABILITY_THRESHOLD.mul_floor(Keys::<T>::get().len() as u32);

			if vote_num as u32 > threshold {
				metadata.is_available = false;
			}

			Ok(())
		})
	}

	// Initialize the authority keys.
	fn initialize_keys(keys: &[T::AuthorityId]) {
		if !keys.is_empty() {
			assert!(Keys::<T>::get().is_empty(), "Keys are already initialized!");
			let bounded_keys = <BoundedSlice<'_, _, T::MaxKeys>>::try_from(keys)
				.expect("More than the maximum number of keys provided");
			Keys::<T>::put(bounded_keys);
		}
	}

	// Set the authority keys (used for testing purposes).
	#[cfg(test)]
	fn set_keys(keys: Vec<T::AuthorityId>) {
		let bounded_keys = WeakBoundedVec::<_, T::MaxKeys>::try_from(keys)
			.expect("More than the maximum number of keys provided");
		Keys::<T>::put(bounded_keys);
	}
}

impl<T: Config> HeaderCommitList for Pallet<T> {
	fn last() -> (Vec<KZGCommitment>, Vec<AppLookup>) {
		let now = <frame_system::Pallet<T>>::block_number();
		if now <= DELAY_CHECK_THRESHOLD.into() {
			(Vec::default(), Vec::default())
		} else {
			Self::get_commitments_and_app_lookups(now - DELAY_CHECK_THRESHOLD.into())
		}
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::AuthorityId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::AuthorityId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
	{
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		Self::initialize_keys(&keys);
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
	{
		// Remember who the authorities are for the new session.
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		let bounded_keys = WeakBoundedVec::<_, T::MaxKeys>::force_from(
			keys,
			Some(
				"Warning: The session has more keys than expected. \
  				A runtime configuration adjustment may be needed.",
			),
		);
		Keys::<T>::put(bounded_keys);
	}

	fn on_before_session_ending() {
		// ingore
	}

	fn on_disabled(i: u32) {
		Keys::<T>::mutate(|keys| {
			keys.remove(i as usize);
		});
	}
}
