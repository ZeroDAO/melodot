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

use futures::{channel::mpsc, Stream};
pub use node_primitives::AccountId;
pub use sc_network::{DhtEvent, KademliaKey, NetworkDHTProvider, NetworkSigner, NetworkStateInfo};
use std::sync::Arc;

pub use crate::{dht_work::Worker, service::Service};

mod dht_work;
mod service;
mod tx_pool_listener;

pub use tx_pool_listener::{start_tx_pool_listener,TPListenerParams};

pub trait NetworkProvider: NetworkDHTProvider + NetworkStateInfo + NetworkSigner {}
impl<T> NetworkProvider for T where T: NetworkDHTProvider + NetworkStateInfo + NetworkSigner {}

pub use melo_core_primitives::{Sidercar, SidercarMetadata, SidercarStatus};
use sp_core::H256;

pub fn new_worker<Client, Network, DhtEventStream>(
	client: Arc<Client>,
	network: Arc<Network>,
	from_service: mpsc::Receiver<ServicetoWorkerMsg>,
	dht_event_rx: DhtEventStream,
) -> Worker<Client, Network, DhtEventStream>
where
	Network: NetworkProvider,
	DhtEventStream: Stream<Item = DhtEvent> + Unpin,
{
	Worker::new(from_service, client, network, dht_event_rx)
}

pub fn new_workgroup() -> (mpsc::Sender<ServicetoWorkerMsg>, mpsc::Receiver<ServicetoWorkerMsg>) {
	mpsc::channel(0)
}

pub fn new_service(to_worker: mpsc::Sender<ServicetoWorkerMsg>) -> Service {
	Service::new(to_worker)
}

pub fn new_worker_and_service<Client, Network, DhtEventStream>(
	client: Arc<Client>,
	network: Arc<Network>,
	dht_event_rx: DhtEventStream,
) -> (Worker<Client, Network, DhtEventStream>, Service)
where
	Network: NetworkProvider,
	DhtEventStream: Stream<Item = DhtEvent> + Unpin,
{
	let (to_worker, from_service) = mpsc::channel(0);

	let worker = Worker::new(from_service, client, network, dht_event_rx);
	let service = Service::new(to_worker);

	(worker, service)
}

pub fn sidercar_kademlia_key(sidercar: &Sidercar) -> KademliaKey {
	KademliaKey::from(Vec::from(sidercar.id()))
}

pub fn kademlia_key_from_sidercar_id(sidercar_id: &H256) -> KademliaKey {
	KademliaKey::from(Vec::from(&sidercar_id[..]))
}

/// Message send from the [`Service`] to the [`Worker`].
pub enum ServicetoWorkerMsg {
	/// See [`Service::get_addresses_by_authority_id`].
	PutValueToDht(KademliaKey, Vec<u8>),
}