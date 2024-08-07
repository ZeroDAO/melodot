//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use futures::lock::Mutex;
use jsonrpsee::RpcModule;
use melo_core_primitives::traits::AppDataApi;
use melo_daser::DasNetworkOperations;

use melo_das_db::traits::DasKv;
use melodot_runtime::{AccountId, Balance, BlockNumber, Hash, NodeBlock as Block, Nonce};

use grandpa::{
	FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use sc_client_api::AuxStore;
use sc_consensus_babe::BabeWorkerHandle;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;
use sp_keystore::KeystorePtr;

use melodot_runtime::RuntimeCall;

pub use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;

/// A type representing all RPC extensions.
pub type RpcExtension = RpcModule<()>;

/// Extra dependencies for BABE.
pub struct BabeDeps {
	/// A handle to the BABE worker for issuing requests.
	pub babe_worker_handle: BabeWorkerHandle<Block>,
	/// The keystore that manages the keys of the node.
	pub keystore: KeystorePtr,
}

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: sc_rpc::SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, SC, B, D, DB> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// A copy of the chain spec.
	pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// Whether to deny unsafe calls
	pub select_chain: SC,
	/// BABE specific dependencies.
	pub babe: BabeDeps,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
	/// DAS network service.
	pub das_network: Arc<D>,
	/// DAS database.
	pub das_db: Arc<Mutex<DB>>,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, SC, B, D, DB>(
	deps: FullDeps<C, P, SC, B, D, DB>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
	C: Send + Sync + 'static,
	C: sc_client_api::BlockBackend<Block> + AuxStore,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: BabeApi<Block>,
	C::Api: BlockBuilder<Block>,
	C::Api: AppDataApi<Block, RuntimeCall>,
	P: TransactionPool<Block = Block> + Sync + Send + 'static,
	SC: SelectChain<Block> + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
	B::State: sc_client_api::backend::StateBackend<sp_runtime::traits::HashingFor<Block>>,
	D: DasNetworkOperations + Sync + Send + 'static + Clone,
	DB: DasKv + Send + Sync + 'static,
{
	use melo_das_rpc::{Confidence, ConfidenceApiServer};
	use melo_das_rpc::{SubmitBlob, SubmitBlobApiServer};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use sc_consensus_babe_rpc::{Babe, BabeApiServer};
	use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
	use sc_sync_state_rpc::{SyncState, SyncStateApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	let mut module = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		chain_spec,
		deny_unsafe,
		select_chain,
		babe,
		grandpa,
		das_network,
		das_db,
	} = deps;

	let BabeDeps { babe_worker_handle, keystore } = babe;
	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;

	module.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	module.merge(
		Babe::new(client.clone(), babe_worker_handle.clone(), keystore, select_chain, deny_unsafe)
			.into_rpc(),
	)?;
	module.merge(
		Grandpa::new(
			subscription_executor,
			shared_authority_set.clone(),
			shared_voter_state,
			justification_stream,
			finality_provider,
		)
		.into_rpc(),
	)?;
	module.merge(
		SyncState::new(chain_spec, client.clone(), shared_authority_set, babe_worker_handle)?
			.into_rpc(),
	)?;

	module.merge(SubmitBlob::new(client.clone(), pool, das_network.clone()).into_rpc())?;

	module.merge(Confidence::<DB, Hash, D>::new(&das_db, &das_network).into_rpc())?;

	// Extend this RPC with a custom API by using the following syntax.
	// `YourRpcStruct` should have a reference to a client, which is needed
	// to call into the runtime.
	// `module.merge(YourRpcTrait::into_rpc(YourRpcStruct::new(ReferenceToClient, ...)))?;`

	Ok(module)
}
