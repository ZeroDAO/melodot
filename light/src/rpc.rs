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

#![warn(missing_docs)]

use jsonrpsee::{server::ServerBuilder, RpcModule};

use futures::lock::Mutex;
use melo_das_db::traits::DasKv;
use melo_daser::DasNetworkOperations;
use meloxt::H256;
use std::{net::SocketAddr, sync::Arc};

pub struct FullDeps<DB, DN> {
	pub db: Arc<Mutex<DB>>,
	pub das_network: Arc<DN>,
}

/// Instantiate all full RPC extensions.
pub fn create_full<DB, DN>(deps: &FullDeps<DB, DN>) -> anyhow::Result<RpcModule<()>>
where
	DB: DasKv + Send + Sync + 'static,
	DN: DasNetworkOperations + Send + Sync + Clone + 'static,
{
	use melo_das_rpc::{Confidence, ConfidenceApiServer};

	let mut module = RpcModule::new(());
	let FullDeps { db, das_network } = deps;

	module.merge(Confidence::<DB, H256, DN>::new(&db.clone(), das_network).into_rpc())?;

	// Extend this RPC with a custom API by using the following syntax.
	// `YourRpcStruct` should have a reference to a client, which is needed
	// to call into the runtime.
	// `module.merge(YourRpcTrait::into_rpc(YourRpcStruct::new(ReferenceToClient, ...)))?;`

	Ok(module)
}

pub async fn run_server<DB, DN>(
	deps: &FullDeps<DB, DN>,
	addre: &SocketAddr,
) -> anyhow::Result<SocketAddr>
where
	DB: DasKv + Send + Sync + 'static,
	DN: DasNetworkOperations + Clone + Send + Sync + 'static,
{
	let module = create_full(deps)?;

	let server = ServerBuilder::default().build(addre).await?;
	let addr = server.local_addr()?;
	let handle = server.start(module.clone())?;

	tokio::spawn(handle.stopped());

	Ok(addr)
}
