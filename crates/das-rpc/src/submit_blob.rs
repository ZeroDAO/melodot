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

use crate::Error;

use codec::{Decode, Encode};
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
};
use log::{error, info};
use melo_core_primitives::traits::AppDataApi;
use melo_daser::DasNetworkOperations;
use melodot_runtime::{RuntimeCall, UncheckedExtrinsic};

use sc_transaction_pool_api::{error::IntoPoolError, TransactionPool, TransactionSource};
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_runtime::traits::Block as BlockT;
use std::{marker::PhantomData, sync::Arc};

/// Represents the status of a Blob transaction.
/// Includes the transaction hash and potential error details.
#[derive(Eq, PartialEq, Default, Clone, Encode, Decode, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlobTxSatus<Hash> {
	pub tx_hash: Hash,
	pub err: Option<String>,
}

/// Defines the Das API's functionalities.
#[rpc(client, server, namespace = "das")]
pub trait SubmitBlobApi<Hash> {
	/// Method for submitting blob transactions.
	/// This will take care of encoding, and then submitting the data and extrinsic to the pool.
	#[method(name = "submitBlobTx")]
	async fn submit_blob_tx(&self, data: Bytes, extrinsic: Bytes) -> RpcResult<BlobTxSatus<Hash>>;
}

/// Main structure representing the Das system.
/// Holds client connection, transaction pool, and DHT network service.
pub struct SubmitBlob<P: TransactionPool, Client, B, D> {
	/// Client interface for interacting with the blockchain.
	client: Arc<Client>,
	/// Pool for managing and processing transactions.
	pool: Arc<P>,
	/// DAS DHT network service.
	das_network: Arc<D>,
	/// Marker for the block type.
	_marker: PhantomData<B>,
}

impl<P: TransactionPool, Client, B, D> SubmitBlob<P, Client, B, D> {
	/// Constructor: Creates a new instance of Das.
	pub fn new(client: Arc<Client>, pool: Arc<P>, das_network: Arc<D>) -> Self {
		Self { client, pool, das_network, _marker: Default::default() }
	}
}

const TX_SOURCE: TransactionSource = TransactionSource::External;

#[async_trait]
impl<P, C, Block, D> SubmitBlobApiServer<P::Hash> for SubmitBlob<P, C, Block, D>
where
	Block: BlockT,
	P: TransactionPool<Block = Block> + 'static,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static + Sync + Send,
	C::Api: AppDataApi<Block, RuntimeCall>,
	D: DasNetworkOperations + Sync + Send + 'static + Clone,
{
	/// Submits a blob transaction to the transaction pool.
	/// The transaction undergoes validation and then gets executed by the runtime.
	///
	/// # Arguments
	/// * `data` - Raw data intended for DHT network.
	/// * `extrinsic` - An unsigned extrinsic to be included in the transaction pool.
	///
	/// # Returns
	/// A struct containing:
	/// * `tx_hash` - The hash of the transaction.
	/// * `err` - `Some` error string if the data submission fails. `None` if successful.
	///
	/// # Note
	/// Ensure proper encoding of the data. Improper encoding can result in a successful transaction
	/// submission (if it's valid), but a failed data publication, rendering the data inaccessible.
	async fn submit_blob_tx(
		&self,
		data: Bytes,
		extrinsic: Bytes,
	) -> RpcResult<BlobTxSatus<P::Hash>> {
		// Decode the provided extrinsic.
		let xt = Decode::decode(&mut &extrinsic[..])
			.map_err(|e| Error::DecodingExtrinsicFailed(Box::new(e)))?;

		let ext = UncheckedExtrinsic::decode(&mut &extrinsic[..])
			.map_err(|e| Error::DecodingTransactionMetadataFailed(Box::new(e)))?;

		// Get block hash
		let at = self.client.info().best_hash;

		// Get blob_tx_param and validate
		let metadata = self
			.client
			.runtime_api()
			.get_blob_tx_param(at, &ext.function)
			.map_err(|e| Error::FetchTransactionMetadataFailed(Box::new(e)))?
			.ok_or(Error::InvalidTransactionFormat)?;

		// Validate the length of the data.
		if !metadata.check() || data.len() != (metadata.bytes_len as usize) {
			return Err(Error::DataLength.into());
		}

		let mut err_msg = None;

		match metadata.verify_bytes(&data) {
			Ok(true) => {
				info!("🤩 Data verification successful. Pushing data to DHT network.");
				// On successful data verification, push data to DHT network.
				let put_res =
					self.das_network.put_bytes(&data, metadata.app_id, metadata.nonce).await;

				if let Err(e) = put_res {
					error!("❌ Failed to put data to DHT network: {:?}", e);
					err_msg = Some(e.to_string());
				}
			},
			Ok(false) => {
				// Handle cases where data verification failed.
				err_msg = Some(
					"Data verification failed. Please check your data and try again.".to_string(),
				);
			},
			Err(e) => {
				// Handle unexpected errors during verification.
				err_msg = Some(e);
			},
		}

		// Submit to the transaction pool
		let best_block_hash = self.client.info().best_hash;
		// let at = generic::BlockId::hash(best_block_hash)
		// 	as generic::BlockId<<P as sc_transaction_pool_api::TransactionPool>::Block>;

		let tx_hash = self.pool.submit_one(best_block_hash, TX_SOURCE, xt).await.map_err(|e| {
			e.into_pool_error()
				.map(|e| Error::TransactionPushFailed(Box::new(e)))
				.unwrap_or_else(|e| Error::TransactionPushFailed(Box::new(e)))
		})?;

		// Return the transaction hash
		Ok(BlobTxSatus { tx_hash, err: err_msg })
	}
}
