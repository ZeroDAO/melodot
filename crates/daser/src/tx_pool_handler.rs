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

//! Transaction pool listener.
//!
//! This module is responsible for monitoring the transaction pool for incoming transactions and
//! processing them accordingly.
//!
//! The transaction pool listener is responsible for the following:
//!
//! - Monitoring the transaction pool for incoming transactions and processing them accordingly.
//! - Monitoring the network for new blocks and processing them accordingly.
//! - Sampling blocks after finalization to determine block data availability.
use crate::{
	Arc, DasKv, DasNetworkOperations, Sampling, SamplingClient, EXTENDED_SEGMENTS_PER_BLOB,
};
use futures::StreamExt;
use log::{error, info, warn};
use melo_core_primitives::{config::BLOCK_SAMPLE_LIMIT, traits::Extractor, Encode};
use sc_client_api::{client::BlockchainEvents, HeaderBackend};
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::{Block as BlockT, NumberFor};
use std::marker::PhantomData;

use futures::stream::FuturesUnordered;
use melo_core_primitives::traits::HeaderWithCommitment;
use sp_api::HeaderT;

// Define a constant for logging with a target string
const LOG_TARGET: &str = "tx_pool_listener";

/// Parameters required for the transaction pool listener.
#[derive(Clone)]
pub struct TPListenerParams<Client, H, TP, DB, D: DasNetworkOperations + std::marker::Sync> {
	pub client: Arc<Client>,
	pub das_client: Arc<SamplingClient<H, DB, D>>,
	pub transaction_pool: Arc<TP>,
	_phantom: PhantomData<DB>,
}

impl<Client, H, TP, DB, D: DasNetworkOperations + std::marker::Sync>
	TPListenerParams<Client, H, TP, DB, D>
{
	pub fn new(
		client: Arc<Client>,
		das_client: Arc<SamplingClient<H, DB, D>>,
		transaction_pool: Arc<TP>,
	) -> Self {
		Self { client, das_client, transaction_pool, _phantom: PhantomData }
	}
}

/// Main function responsible for starting the transaction pool listener.
/// It monitors the transaction pool for incoming transactions and processes them accordingly.
pub async fn start_tx_pool_listener<
	Client,
	TP,
	B,
	DB,
	H,
	D: DasNetworkOperations + std::marker::Sync,
>(
	TPListenerParams { client, das_client, transaction_pool, _phantom }: TPListenerParams<
		Client,
		H,
		TP,
		DB,
		D,
	>,
) where
	TP: TransactionPool<Block = B> + 'static,
	B: BlockT + Send + Sync + 'static,
	<B as BlockT>::Header: HeaderWithCommitment,
	Client: ProvideRuntimeApi<B> + HeaderBackend<B> + BlockchainEvents<B> + 'static,
	Client::Api: Extractor<B>,
	DB: DasKv + 'static + Send + Sync,
	H: HeaderWithCommitment + Send + Sync + 'static,
	NumberFor<B>: Into<u32>,
{
	info!("🚀 Starting transaction pool listener.");

	let mut import_notification_stream = transaction_pool.import_notification_stream();
	let mut new_best_block_stream = client.import_notification_stream();
	let mut finality_notification_stream = client.finality_notification_stream();

	loop {
		tokio::select! {
			Some(notification) = import_notification_stream.next() => {
				// Process ready transactions in the transaction pool
				// TODO: Handle cases where the data is still not reached
				if let Some(transaction) = transaction_pool.ready_transaction(&notification) {
					let encoded = transaction.data().encode();
					let at = client.info().best_hash;

					// Extract relevant information from the encoded transaction data
					match client.runtime_api().extract(at, &encoded) {
						Ok(Some(data)) => {
							for params in data {
								tracing::debug!(
									target: LOG_TARGET,
									"New blob transaction found. Hash: {:?}", at,
								);

								if let Err(e) = das_client
									.sample_application(params.app_id, params.nonce, &params.commitments)
									.await
								{
									warn!("⚠️ Error during sampling application: {:?}", e);
									continue;
								}
							}
						},
						Ok(None) => tracing::debug!(
							target: LOG_TARGET,
							"Decoding of extrinsic failed. Transaction: {:?}",
							transaction.hash(),
						),
						Err(err) => tracing::debug!(
							target: LOG_TARGET,
							"Failed to extract data from extrinsic. Transaction: {:?}. Error: {:?}",
							transaction.hash(),
							err,
						),
					};
				}
			},
			// Restore and extend the best block's data and broadcast the extended data to the network
			// TODO: To be run distributedly by farmers
			Some(notification) = new_best_block_stream.next() => {
				if !notification.is_new_best { continue; }
				let header = notification.header;
				let block_number = HeaderT::number(&header);

				if let Some(cmts) = header.commitments() {
					if cmts.is_empty() {
						info!("😴 Block {} has no blob", block_number);
						continue;
					}
				} else {
					error!("⚠️ Block {} has no commitments information", block_number);
					continue;
				}

				let fetch_result = das_client.network.fetch_block(&header).await;
				let (segments, is_availability) = match fetch_result {
					Ok(data) => data,
					Err(e) => {
						tracing::error!(target: LOG_TARGET, "Error fetching block: {:?}", e);
						continue;
					},
				};

				if !is_availability {
					info!("🥵 Block {} is not available", block_number);
					continue
				}

				for x in 0..EXTENDED_SEGMENTS_PER_BLOB {
					match full_col(&segments, x, EXTENDED_SEGMENTS_PER_BLOB) {
						Ok(col) => {
							match das_client.network.extend_segments_col(&col) {
								Ok(col_ext) => {
									if let Err(e) = das_client.network.put_ext_segments(&col_ext, &header).await {
										error!("⚠️ Error pushing values: {:?}", e);
									}
								},
								Err(e) => {
									error!("⚠️ Error extending col: {:?}", e);
								},
							};
						},
						Err(err) => {
							error!("Column {:?} fetch or reconstruction error: {:?}", x, err);
							continue;
						},
					};
				};
				info!("🎉 Block {} is available", block_number);
			},
			// Sample blocks after finalization to determine block data availability
			// TODO: Sync progress from runtime to eliminate uncertainty in local sampling
			Some(notification) = finality_notification_stream.next() => {
				let header = notification.header;
				let block_number = *HeaderT::number(&header);
				let latest_sampled_block = das_client.last_at().await;

				let to_block = std::cmp::min(
					block_number,
					std::cmp::max(
						(latest_sampled_block + BLOCK_SAMPLE_LIMIT).into(),
						BLOCK_SAMPLE_LIMIT.into()
					)
				);

				let mut i: NumberFor<B> = (latest_sampled_block + 1u32).into();
				let mut sampling_tasks = FuturesUnordered::new();

				while i < to_block {
					match client.hash(i) {
						Ok(Some(block_hash)) => {
							match client.header(block_hash) {
								Ok(header_option) => {
									if let Some(header) = header_option {
										let das_client_clone = das_client.clone();
										sampling_tasks.push(async move {
											das_client_clone.sample_block(&header).await
										});
									}
								},
								Err(e) => {
									tracing::error!(target: LOG_TARGET, "Error getting header for hash {:?}: {:?}", block_hash, e);
								}
							};
						},
						Ok(None) => {
							tracing::error!(target: LOG_TARGET, "No hash found for block number {}", i);
						},
						Err(e) => {
							tracing::error!(target: LOG_TARGET, "Error getting hash for block number {}: {:?}", i, e);
						}
					}

					i += 1u32.into();
				}

				while let Some(result) = sampling_tasks.next().await {
					if let Err(e) = result {
						tracing::error!(target: LOG_TARGET, "Error sampling block: {:?}", e);
					}
				}
			}
		}
	}
}

fn full_col<T>(segments: &[Option<T>], index: usize, len: usize) -> Result<Vec<T>, String>
where
	T: Clone,
{
	segments
		.iter()
		.skip(index)
		.step_by(len)
		.try_fold(Vec::new(), |mut col, maybe_segment| {
			match maybe_segment {
				Some(segment) => col.push(segment.clone()),
				None => return Err("Col is not available".into()),
			}
			Ok(col)
		})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_full_col_success() {
		// 1 2
		// 3 4
		// 5 6
		let segments = vec![Some(1), Some(2), Some(3), Some(4), Some(5), Some(6)];
		let result = full_col(&segments, 0, 2);
		assert_eq!(result, Ok(vec![1, 3, 5]));
	}

	#[test]
	fn test_full_col_missing_data() {
		let segments = vec![Some(1), None, Some(3), Some(4)];
		let result = full_col(&segments, 1, 2);
		assert!(result.is_err());
	}
}
