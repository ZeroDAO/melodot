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

use frame_system::Config;
use futures::StreamExt;
use melo_das_primitives::crypto::{KZGCommitment, KZGProof};
use sc_network::{KademliaKey, NetworkDHTProvider};
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use sp_core::H256;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::traits::Extrinsic;
use std::sync::Arc;

use crate::AccountId;

fn sidercar_kademlia_key(sidercar: &Sidercar) -> KademliaKey {
	KademliaKey::from(Vec::from(sidercar.id()))
}

use crate::{NetworkProvider, Sidercar, SidercarMetadata};

/// Extracts the `data` field from some types of extrinsics.
pub trait Extractor<T: Extrinsic, AccountId> {
	fn extract(
		app_ext: T,
	) -> Option<(H256, u32, u32, Vec<KZGCommitment>, Vec<KZGProof>, AccountId)>;
}

pub async fn process_tx_pool_notifications<Network, TP, B, C>(
	transaction_pool: TP,
	network: Arc<Network>,
) where
	B: BlockT + 'static,
	Network: NetworkProvider,
	TP: TransactionPool<Block = B>,
	C: Config + Send + Sync + Extractor<B::Extrinsic, AccountId>,
{
	// Obtain the import notification event stream from the transaction pool
	let mut import_notification_stream = transaction_pool.import_notification_stream();

	// Handle the transaction pool import notification event stream
	while let Some(notification) = import_notification_stream.next().await {
		match transaction_pool.ready_transaction(&notification) {
			Some(transaction) => {
				let extrinsic = transaction.data();
				if let Some((data_hash, bytes_len, _, commitments, proofs, _)) =
					C::extract(extrinsic.clone())
				{
					let metadata = SidercarMetadata {
						data_len: bytes_len,
						blobs_hash: data_hash,
						commitments,
						proofs,
					};

					let fetch_value_from_network = |sidercar: &Sidercar| {
						network.get_value(&sidercar_kademlia_key(sidercar));
					};

					match Sidercar::from_local(&metadata.id()) {
						Some(sidercar) => {
							if sidercar.status.is_none() {
								fetch_value_from_network(&sidercar);
							}
						},
						None => {
							let sidercar = Sidercar { blobs: None, metadata, status: None };
							sidercar.save_to_local();
							fetch_value_from_network(&sidercar);
						},
					}
				}
			},
			None => {},
		}
	}
}
