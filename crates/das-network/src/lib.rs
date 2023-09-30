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

// Necessary imports for the module.
use futures::{
	channel::{mpsc, oneshot},
	Stream,
};
// Logging macro.
pub use log::warn;
// Common primitives and traits.
pub use node_primitives::AccountId;
pub use sc_client_api::Backend;
pub use sc_network::{DhtEvent, KademliaKey, NetworkDHTProvider, NetworkSigner, NetworkStateInfo};
pub use sc_offchain::OffchainDb;
pub use sp_runtime::traits::{Block, Header};
pub use std::sync::Arc;

// Internal module imports.
pub use crate::{dht_work::Worker, service::Service};

mod dht_work;
mod service;
mod tx_pool_listener;

pub use tx_pool_listener::{start_tx_pool_listener, TPListenerParams};

/// Trait to encapsulate necessary network-related operations.
pub trait NetworkProvider: NetworkDHTProvider + NetworkStateInfo + NetworkSigner {}
impl<T> NetworkProvider for T where T: NetworkDHTProvider + NetworkStateInfo + NetworkSigner {}

// Import core primitives related to sidecars.
pub use melo_core_primitives::{Sidecar, SidecarMetadata, SidecarStatus};
use sp_core::H256;

/// Instantiates a new DHT Worker with the given parameters.
pub fn new_worker<B, Client, Network, DhtEventStream, BE>(
	client: Arc<Client>,
	network: Arc<Network>,
	backend: Arc<BE>,
	from_service: mpsc::Receiver<ServicetoWorkerMsg>,
	dht_event_rx: DhtEventStream,
) -> Option<Worker<B, Client, Network, DhtEventStream, BE>>
where
	B: Block,
	Network: NetworkProvider,
	DhtEventStream: Stream<Item = DhtEvent> + Unpin,
	BE: Backend<B>,
{
	Worker::try_build(from_service, client, backend, network, dht_event_rx)
}

/// Creates a new channel for communication between the service and worker.
pub fn new_workgroup() -> (mpsc::Sender<ServicetoWorkerMsg>, mpsc::Receiver<ServicetoWorkerMsg>) {
	mpsc::channel(0)
}

/// Initializes a new Service instance with the specified communication channel.
pub fn new_service(to_worker: mpsc::Sender<ServicetoWorkerMsg>) -> Service {
	Service::new(to_worker)
}

/// Conveniently creates both a Worker and Service with the given parameters.
pub fn new_worker_and_service<B, Client, Network, DhtEventStream, BE>(
	client: Arc<Client>,
	network: Arc<Network>,
	dht_event_rx: DhtEventStream,
	backend: Arc<BE>,
) -> Option<(Worker<B, Client, Network, DhtEventStream, BE>, Service)>
where
	B: Block,
	Network: NetworkProvider,
	DhtEventStream: Stream<Item = DhtEvent> + Unpin,
	BE: Backend<B>,
{
	let (to_worker, from_service) = mpsc::channel(0);

	let worker = Worker::try_build(from_service, client, backend, network, dht_event_rx)?;
	let service = Service::new(to_worker);

	Some((worker, service))
}

/// Converts a sidecar instance into a Kademlia key.
pub fn sidecar_kademlia_key(sidecar: &Sidecar) -> KademliaKey {
	KademliaKey::from(Vec::from(sidecar.id()))
}

/// Converts a sidecar ID into a Kademlia key.
pub fn kademlia_key_from_sidecar_id(sidecar_id: &H256) -> KademliaKey {
	KademliaKey::from(Vec::from(&sidecar_id[..]))
}

/// Enumerated messages that can be sent from the Service to the Worker.
pub enum ServicetoWorkerMsg {
	/// Request to insert a value into the DHT.
	/// Contains the key for insertion, the data to insert, and a sender to acknowledge completion.
	PutValueToDht(KademliaKey, Vec<u8>, oneshot::Sender<Option<()>>),
}
