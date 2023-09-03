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

mod error;

use codec::Decode;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
};
use melo_core_primitives::traits::AppDataApi;
use melo_core_primitives::{Sidercar, SidercarMetadata};
use melo_das_network::{kademlia_key_from_sidercar_id, NetworkProvider};
use melodot_runtime::{RuntimeCall, UncheckedExtrinsic};
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::{error::IntoPoolError, TransactionPool, TransactionSource};
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_runtime::{generic, traits::Block as BlockT};
use std::sync::Arc;

use error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlobTxSatus<Hash> {
	tx_hash: Hash,
	err: Option<String>,
}

#[rpc(client, server)]
pub trait BlobApi<Hash> {
	#[method(name = "blob_submit")]
	async fn submit_blob_tx(&self, data: Bytes, extrinsic: Bytes) -> RpcResult<BlobTxSatus<Hash>>;
}

pub struct Das<TP, Client, Network> {
	/// Substrate client
	client: Arc<Client>,
	/// Transactions pool
	pool: Arc<TP>,
	/// DHT network
	pub network: Arc<Network>,
}

const TX_SOURCE: TransactionSource = TransactionSource::External;
#[async_trait]
impl<TP, Client, Network> BlobApiServer<TP::Hash> for Das<TP, Client, Network>
where
	Network: NetworkProvider + Send + Sync + 'static,
	TP: TransactionPool + Sync + Send + 'static,
	Client: HeaderBackend<TP::Block> + ProvideRuntimeApi<TP::Block> + Send + Sync + 'static,
	TP::Hash: Unpin,
	<TP::Block as BlockT>::Hash: Unpin,
	Client::Api: AppDataApi<TP::Block, RuntimeCall>,
{
	async fn submit_blob_tx(
		&self,
		data: Bytes,
		extrinsic: Bytes,
	) -> RpcResult<BlobTxSatus<TP::Hash>> {
		// Decode the extrinsic
		let xt = Decode::decode(&mut &extrinsic[..]).map_err(|e| Error::DecodingExtrinsicFailed(Box::new(e)))?;

		let ext = UncheckedExtrinsic::decode(&mut &extrinsic[..])
			.map_err(|e| Error::DecodingTransactionMetadataFailed(Box::new(e)))?;

		// Get block hash
		let at = self.client.info().best_hash;

		// Get blob_tx_param and validate
		let (data_hash, data_len, commitments, proofs) = self
			.client
			.runtime_api()
			.get_blob_tx_param(at, &ext.function)
			.map_err(|e| Error::FetchTransactionMetadataFailed(Box::new(e)))?
			.ok_or(Error::InvalidTransactionFormat)?;

		// Validate data_len and data_hash
		if data_len != (data.len() as u32) || Sidercar::calculate_id(&data)[..] != data_hash[..] {
			return Err(Error::DataLengthOrHashError.into());
		}

		// Submit to the transaction pool
		let best_block_hash = self.client.info().best_hash;
		let tx_hash = self
			.pool
			.submit_one(&generic::BlockId::hash(best_block_hash), TX_SOURCE, xt)
			.await
			.map_err(|e| {
				e.into_pool_error()
					.map(|e| Error::TransactionPushFailed(Box::new(e)))
					.unwrap_or_else(|e| Error::TransactionPushFailed(Box::new(e)).into())
			})?;

		let metadata = SidercarMetadata { data_len, blobs_hash: data_hash, commitments, proofs };

		let mut blob_tx_status = BlobTxSatus { tx_hash, err: None };

		match metadata.verify_bytes(&data) {
			Ok(true) => {
				// Push data to the DHT network
				self.network.put_value(kademlia_key_from_sidercar_id(&data_hash), data.to_vec());
			},
			Ok(false) => {
				blob_tx_status.err = Some(
					"Data verification failed. Please check your data and try again.".to_string(),
				);
			},
			Err(e) => {
				blob_tx_status.err = Some(e);
			},
		}

		// Return the transaction hash
		Ok(blob_tx_status)
	}
}
