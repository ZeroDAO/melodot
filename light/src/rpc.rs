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
use meloxt::H256;
use std::{net::SocketAddr, sync::Arc};

pub struct FullDeps<DB> {
	pub db: Arc<Mutex<DB>>,
}

/// Instantiate all full RPC extensions.
pub fn create_full<DB>(deps: &FullDeps<DB>) -> anyhow::Result<RpcModule<()>>
where
	DB: DasKv + Send + Sync + 'static,
{
	use melo_das_rpc::{Confidence, ConfidenceApiServer};

	let mut module = RpcModule::new(());
	let FullDeps { db } = deps;

	module.merge(Confidence::<DB, H256>::new(&db.clone()).into_rpc())?;

	// Extend this RPC with a custom API by using the following syntax.
	// `YourRpcStruct` should have a reference to a client, which is needed
	// to call into the runtime.
	// `module.merge(YourRpcTrait::into_rpc(YourRpcStruct::new(ReferenceToClient, ...)))?;`

	Ok(module)
}

pub async fn run_server<DB>(deps: &FullDeps<DB>, addre: &SocketAddr) -> anyhow::Result<SocketAddr>
where
	DB: DasKv + Send + Sync + 'static,
{
	let module = create_full(deps)?;

	let server = ServerBuilder::default().build(addre).await?;
	let addr = server.local_addr()?;
	let handle = server.start(module.clone())?;

	tokio::spawn(handle.stopped());

	Ok(addr)
}
