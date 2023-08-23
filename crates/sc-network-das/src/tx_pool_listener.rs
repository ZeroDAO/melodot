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
use node_primitives::AccountId;
use sc_network::NetworkDHTProvider;
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

use crate::{
	get_sidercar_from_localstorage, save_sidercar_to_localstorage, Extractor, NetworkProvider,
	Sidercar, SidercarMetadata,
};

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
				if let Some((data_hash, bytes_len, _, commitments, proofs, from)) =
					C::extract(extrinsic.clone())
				{
					let metadata = SidercarMetadata {
						data_len: bytes_len,
						blobs_hash: data_hash,
						commitments,
						proofs,
					};

					match get_sidercar_from_localstorage(&metadata.id()) {
						Some(sidercar) => {
							if sidercar.status.is_none() {
								network.get_value(&sidercar.kademlia_key());
							}
						},
						None => {
							let sidercar = Sidercar { from, blobs: None, metadata, status: None };
							save_sidercar_to_localstorage(sidercar.clone());
							network.get_value(&sidercar.kademlia_key());
						},
					}
				}
			},
			None => {},
		}
	}
}
