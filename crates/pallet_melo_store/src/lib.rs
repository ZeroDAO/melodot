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

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;
pub use weights::*;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	pallet_prelude::*,
	traits::{Get, OneSessionHandler, ValidatorSetWithIdentification},
	BoundedSlice, WeakBoundedVec,
};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
use melo_core_primitives::blob::Blob;
use melo_core_primitives::config::BYTES_PER_BLOB;
pub use pallet::*;
use scale_info::TypeInfo;
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::{
	offchain::storage::{MutateStorageError, StorageRetrievalError, StorageValueRef},
	traits::AtLeast32BitUnsigned,
	Permill, RuntimeDebug,
};
use sp_std::prelude::*;

use melo_core_primitives::kzg::{KZGCommitment, KZGProof};
use sc_network_das::get_sidercar_from_localstorage;

const DB_PREFIX: &[u8] = b"melodot/melo-store/unavailable-data-report";
// 延迟确认不可用的阈值
const DELAY_CHECK_THRESHOLD: u32 = 1;

pub type AuthIndex = u32;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
struct ReportStatus<BlockNumber> {
	/// The block in which the data is reported
	pub at_block: BlockNumber,
	pub sent_at: BlockNumber,
}

impl<BlockNumber: PartialEq + AtLeast32BitUnsigned + Copy> ReportStatus<BlockNumber> {
	fn is_recent(&self, at_block: BlockNumber, now: BlockNumber) -> bool {
		self.at_block == at_block && self.sent_at + DELAY_CHECK_THRESHOLD.into() > now
	}
}

/// Error which may occur while executing the off-chain code.
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

type OffchainResult<T, A> = Result<A, OffchainErr<BlockNumberFor<T>>>;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct UnavailableDataReport<BlockNumber>
where
	BlockNumber: PartialEq + Eq + Decode + Encode,
{
	/// Block number at the time report is created..
	pub at_block: BlockNumber,
	/// An index of the authority on the list of validators.
	pub authority_index: AuthIndex,

	pub index_set: Vec<u32>,
	/// The length of session validator set
	pub validators_len: u32,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use sp_core::H256;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	pub type KZGCommitmentListFor<T> = BoundedVec<KZGCommitment, <T as Config>::MaxBlobNum>;

	#[derive(Clone, Eq, Default, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct BlobMetadata<T: Config> {
		pub app_id: u32,
		pub from: T::AccountId,
		pub commitments: KZGCommitmentListFor<T>,
		pub bytes_len: u32,
		pub data_hash: H256,
		pub is_available: bool,
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: SendTransactionTypes<Call<Self>> + frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// A type for retrieving the validators supposed to be online in a session.
		type ValidatorSet: ValidatorSetWithIdentification<Self::AccountId>;
		/// The identifier type for an authority.
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// The maximum number of keys that can be added.
		type MaxKeys: Get<u32>;

		type MaxBlobNum: Get<u32>;

		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;
	}

	// Store metadata of AppData
	// TODO: We currently don't delete it because, when farmers submit solutions later, if the
	// data is not deleted, it will first validate the Root and then delete the data.
	#[pallet::storage]
	#[pallet::getter(fn metadata)]
	pub(super) type Metadata<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		WeakBoundedVec<BlobMetadata<T>, T::MaxKeys>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn keys)]
	pub(super) type Keys<T: Config> =
		StorageValue<_, WeakBoundedVec<T::AuthorityId, T::MaxKeys>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn app_id)]
	pub(super) type AppId<T: Config> = StorageValue<_, u32, ValueQuery>;

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

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Data has been received
		DataReceived {
			/// Hash of the data
			data_hash: H256,
			/// Length of the data
			bytes_len: u32,
			/// Submitter of the data
			from: T::AccountId,
			/// App ID of the data
			app_id: u32,
			/// Index of the data
			index: u32,
			/// Commitments
			commitments: Vec<KZGCommitment>,
			/// Proofs
			proofs: Vec<KZGProof>,
		},

		/// Report has been received
		ReportReceived {
			/// Block number of the report
			at_block: BlockNumberFor<T>,
			/// Node that submitted the report
			from: AuthIndex,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Exceeds the maximum limit for the number of Blobs.
		ExceedMaxBlobLimit,
		/// There's an error with the app ID.
		AppIdError,
		/// Exceeds the maximum number of Blobs for a single block.
		ExceedMaxBlobPerBlock,
		/// Exceeds the confirmation time for unavailable data.
		ExceedUnavailableDataConfirmTime,
		/// The index set is empty.
		IndexSetIsEmpty,
		/// A duplicate item exists.
		DuplicateItemExists,
		/// The data doesn't exist.
		DataNotExist,
		/// Metadata modification is not allowed.
		CannotModifyMetadata,
		/// The report has been submitted more than once.
		DuplicateReportSubmission,
		/// Exceeds the maximum total votes.
		ExceedMaxTotalVotes,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::default())]
		pub fn submit_data(
			origin: OriginFor<T>,
			app_id: u32,
			bytes_len: u32,
			data_hash: H256,
			commitments: Vec<KZGCommitment>,
			proofs: Vec<KZGProof>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let blob_num = Blob::blob_count(bytes_len as usize, BYTES_PER_BLOB);
			ensure!(blob_num <= T::MaxBlobNum::get() as usize, Error::<T>::ExceedMaxBlobLimit);

			let current_app_id = AppId::<T>::get();
			ensure!(app_id <= current_app_id + 1, Error::<T>::AppIdError);

			let mut commitment_list: BoundedVec<KZGCommitment, T::MaxBlobNum> =
				BoundedVec::default();
			commitment_list
				.try_extend(commitments.iter().cloned())
				.map_err(|_| Error::<T>::ExceedMaxBlobPerBlock)?;

			let metadata: BlobMetadata<T> = BlobMetadata {
				app_id,
				from: who.clone(),
				commitments: commitment_list,
				bytes_len,
				data_hash,
				// Theoretically, the submitted data is likely to be available,
				// so we initially assume it's available.
				is_available: true,
			};

			let current_block_number = <frame_system::Pallet<T>>::block_number();

			let mut metadata_len = 0;
			Metadata::<T>::try_mutate(current_block_number, |metadata_vec| {
				metadata_len = metadata_vec.len();
				metadata_vec.try_push(metadata)
			})
			.map_err(|_| Error::<T>::ExceedMaxBlobPerBlock)?;

			if app_id == current_app_id + 1 {
				AppId::<T>::put(app_id);
			}

			Self::deposit_event(Event::DataReceived {
				data_hash,
				bytes_len,
				from: who,
				app_id,
				index: metadata_len as u32,
				commitments,
				proofs,
			});

			Ok(().into())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::default())]
		pub fn report(
			origin: OriginFor<T>,
			unavailable_data_report: UnavailableDataReport<BlockNumberFor<T>>,
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;
			let current_block_number: BlockNumberFor<T> = <frame_system::Pallet<T>>::block_number();

			ensure!(
				unavailable_data_report.at_block + DELAY_CHECK_THRESHOLD.into()
					== current_block_number,
				Error::<T>::ExceedUnavailableDataConfirmTime
			);
			ensure!(!unavailable_data_report.index_set.is_empty(), Error::<T>::IndexSetIsEmpty);

			let mut index_set = unavailable_data_report.index_set;
			index_set.sort();

			Metadata::<T>::try_mutate(
				unavailable_data_report.at_block,
				|metadata_vec| -> DispatchResult {
					let metadata_vec_mut = metadata_vec;
					for window in index_set.windows(2) {
						// Check for duplication
						ensure!(window[0] != window[1], Error::<T>::DuplicateItemExists);
						let index = window[0];
						let metadata = metadata_vec_mut
							// An error is thrown if the boundary is crossed
							.get_mut(index as usize)
							.ok_or(Error::<T>::DataNotExist)?;

						if metadata.is_available {
							<Pallet<T>>::handle_vote(
								unavailable_data_report.at_block,
								unavailable_data_report.authority_index,
								metadata,
							)?;
						}
					}
					Self::deposit_event(Event::ReportReceived {
						at_block: unavailable_data_report.at_block,
						from: unavailable_data_report.authority_index,
					});
					Ok(())
				},
			)
			.map_err(|_| Error::<T>::CannotModifyMetadata.into())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(now: BlockNumberFor<T>) {
			// Deletion of expired polling data
			if T::BlockNumber::from(DELAY_CHECK_THRESHOLD + 1) >= now {
				return;
			}
			let _ =
				UnavailableVote::<T>::clear_prefix(now - (DELAY_CHECK_THRESHOLD + 1).into(), T::MaxBlobNum::get(), None);
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
			} else {
				log::trace!(
					target: "runtime::melo-store",
					"Skipping report at {:?}. Not a validator.",
					now,
				)
			}
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::report { unavailable_data_report, signature } = call {
				let keys = Keys::<T>::get();

				let authority_id =
					match keys.get(unavailable_data_report.authority_index.clone() as usize) {
						Some(id) => id,
						None => return InvalidTransaction::BadProof.into(),
					};

				let signature_valid = unavailable_data_report.using_encoded(|encoded_report| {
					authority_id.verify(&encoded_report, signature)
				});

				if !signature_valid {
					return InvalidTransaction::BadProof.into();
				}

				ValidTransaction::with_tag_prefix("MeloStore")
					.priority(T::UnsignedPriority::get())
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
	pub fn get_unavailability_data(at_block: BlockNumberFor<T>) -> Vec<u32> {
		Metadata::<T>::get(at_block)
			.iter()
			.enumerate()
			.filter_map(|(i, metadata)| {
				if let Some(sidercar) =
					get_sidercar_from_localstorage(&metadata.data_hash.as_bytes())
				{
					if sidercar.is_unavailability() {
						Some(i as u32)
					} else {
						None
					}
				} else {
					None
				}
			})
			.collect::<Vec<_>>()
	}

	pub fn get_commitment_list(at_block: BlockNumberFor<T>) -> Vec<KZGCommitment> {
		Metadata::<T>::get(at_block)
			.iter()
			.map(|metadata| metadata.commitments.clone())
			.flatten()
			.collect::<Vec<_>>()
	}

	pub(crate) fn send_unavailability_report(
		now: BlockNumberFor<T>,
	) -> OffchainResult<T, impl Iterator<Item = OffchainResult<T, ()>>> {
		let reports = (0..DELAY_CHECK_THRESHOLD)
			.into_iter()
			.filter_map(move |gap| {
				if T::BlockNumber::from(gap) > now {
					return None;
				}
				let at_block = now - gap.into();
				let index_set = Self::get_unavailability_data(at_block);
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
			.flat_map(|it| it);

		Ok(reports)
	}

	fn send_single_unavailability_report(
		authority_index: u32,
		key: T::AuthorityId,
		at_block: BlockNumberFor<T>,
		now: BlockNumberFor<T>,
		index_set: Vec<u32>,
	) -> OffchainResult<T, ()> {
		let prepare_unavailable_data_report = || -> OffchainResult<T, Call<T>> {
			let unavailable_data_report = UnavailableDataReport {
				at_block,
				authority_index,
				index_set,
				validators_len: Keys::<T>::get().len() as u32,
			};

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

	fn with_report_lock<R>(
		authority_index: u32,
		at_block: BlockNumberFor<T>,
		now: BlockNumberFor<T>,
		f: impl FnOnce() -> OffchainResult<T, R>,
	) -> OffchainResult<T, R> {
		let key = {
			let mut key = DB_PREFIX.to_vec();
			key.extend(authority_index.encode());
			key
		};
		let storage = StorageValueRef::persistent(&key);
		let res = storage.mutate(
			|status: Result<Option<ReportStatus<BlockNumberFor<T>>>, StorageRetrievalError>| {
				match status {
					// we are still waiting for inclusion.
					Ok(Some(status)) if status.is_recent(at_block, now) => {
						Err(OffchainErr::WaitingForInclusion(status.sent_at))
					},
					// attempt to set new status
					_ => Ok(ReportStatus { at_block, sent_at: now }),
				}
			},
		);
		if let Err(MutateStorageError::ValueFunctionFailed(err)) = res {
			return Err(err);
		}

		let mut new_status = res.map_err(|_| OffchainErr::FailedToAcquireLock)?;

		let res = f();

		if res.is_err() {
			new_status.sent_at = 0u32.into();
			storage.set(&new_status);
		}

		res
	}

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

	fn handle_vote(
		at_block: BlockNumberFor<T>,
		authority_index: AuthIndex,
		metadata: &mut BlobMetadata<T>,
	) -> Result<(), DispatchError> {
		const UNAVAILABILITY_THRESHOLD: Permill = Permill::from_percent(50);

		UnavailableVote::<T>::try_mutate(at_block, metadata.app_id, |maybe_unavailable_vote| {
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

	fn initialize_keys(keys: &[T::AuthorityId]) {
		if !keys.is_empty() {
			assert!(Keys::<T>::get().is_empty(), "Keys are already initialized!");
			let bounded_keys = <BoundedSlice<'_, _, T::MaxKeys>>::try_from(keys)
				.expect("More than the maximum number of keys provided");
			Keys::<T>::put(bounded_keys);
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
