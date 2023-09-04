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
use melo_das_network::kademlia_key_from_sidercar_id;
use melo_das_network_protocol::DasDht;
use melodot_runtime::{RuntimeCall, UncheckedExtrinsic};

use sc_transaction_pool_api::{error::IntoPoolError, TransactionPool, TransactionSource};
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_runtime::{generic, traits::Block as BlockT};
use std::sync::Arc;

pub use sc_rpc_api::DenyUnsafe;

use error::Error;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BlobTxSatus<Hash> {
	tx_hash: Hash,
	err: Option<String>,
}

#[rpc(client, server)]
pub trait DasApi<Hash> {
	#[method(name = "das_submitBlobTx")]
	async fn submit_blob_tx(&self, data: Bytes, extrinsic: Bytes) -> RpcResult<BlobTxSatus<Hash>>;
}

pub struct Das<P: TransactionPool, Client, DDS, B> {
	/// Substrate client
	client: Arc<Client>,
	/// Transactions pool
	pool: Arc<P>,
	/// DHT network
	pub service: DDS,
	_marker: std::marker::PhantomData<B>,
}

impl<P: TransactionPool, Client, DDS, B> Das<P, Client, DDS, B> {
	/// Creates a new instance of the Das Rpc helper.
	pub fn new(client: Arc<Client>, pool: Arc<P>, service: DDS) -> Self {
		Self { client, pool, service, _marker: Default::default() }
	}
}

const TX_SOURCE: TransactionSource = TransactionSource::External;

#[async_trait]
impl<P, C, DDS, Block> DasApiServer<P::Hash> for Das<P, C, DDS, Block>
where
	Block: BlockT,
	P: TransactionPool<Block = Block> + 'static,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static + Sync + Send,
	C::Api: AppDataApi<Block, RuntimeCall>,
	DDS: DasDht + Sync + Send + 'static + Clone,
{
	async fn submit_blob_tx(
		&self,
		data: Bytes,
		extrinsic: Bytes,
	) -> RpcResult<BlobTxSatus<P::Hash>> {
		// Decode the extrinsic
		let xt = Decode::decode(&mut &extrinsic[..])
			.map_err(|e| Error::DecodingExtrinsicFailed(Box::new(e)))?;

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
		let at = generic::BlockId::hash(best_block_hash)
			as generic::BlockId<<P as sc_transaction_pool_api::TransactionPool>::Block>;

		let tx_hash =
			self.pool.submit_one(&at, TX_SOURCE, xt).await.map_err(|e| {
				e.into_pool_error()
					.map(|e| Error::TransactionPushFailed(Box::new(e)))
					.unwrap_or_else(|e| Error::TransactionPushFailed(Box::new(e)).into())
			})?;

		let metadata = SidercarMetadata { data_len, blobs_hash: data_hash, commitments, proofs };

		let mut blob_tx_status = BlobTxSatus { tx_hash, err: None };

		match metadata.verify_bytes(&data) {
			Ok(true) => {
				// Push data to the DHT network
				let mut dht_service = self.service.clone();
				let put_res = dht_service
					.put_value_to_dht(kademlia_key_from_sidercar_id(&data_hash), data.to_vec())
					.await
					.is_some();
				if !put_res {
					blob_tx_status.err = Some("Failed to put data to DHT network.".to_string());
				}
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
