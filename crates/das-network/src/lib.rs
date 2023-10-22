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

use anyhow::{Ok, Result};
use futures::channel::mpsc;
use libp2p::{
	core::{
		muxing::StreamMuxerBox,
		transport::{self},
		upgrade::Version,
		PeerId,
	},
	dns::TokioDnsConfig,
	identify::Config as IdentifyConfig,
	identity,
	identity::Keypair,
	kad::{store::MemoryStore, KademliaConfig},
	metrics::Metrics,
	noise::NoiseAuthenticated,
	swarm::{Swarm, SwarmBuilder},
	tcp::Config as GenTcpConfig,
	yamux::YamuxConfig,
	Transport,
};
use libp2p::tcp::tokio::Transport as TokioTcpTransport;

pub use log::warn;

pub use node_primitives::AccountId;
pub use sc_client_api::Backend;
pub use sc_network::{DhtEvent, KademliaKey, NetworkDHTProvider, NetworkSigner, NetworkStateInfo};
pub use sp_runtime::traits::{Block, Header};
pub use std::sync::Arc;
use std::time::Duration;

pub use behaviour::{Behavior, BehaviorConfig, BehaviourEvent};
pub use service::Service;
pub use shared::Command;
pub use worker::DasNetwork;

mod behaviour;
mod service;
mod shared;
mod tx_pool_listener;
mod worker;

pub use tx_pool_listener::{start_tx_pool_listener, TPListenerParams};

pub fn create(
	keypair: identity::Keypair,
	protocol_version: String,
	metrics: Metrics,
) -> Result<(service::Service, worker::DasNetwork)> {
	let local_peer_id = PeerId::from(keypair.public());

	let protocol_version = format!("/melodot-das/{}", protocol_version);
	let identify = IdentifyConfig::new(protocol_version.clone(), keypair.public());

	// Create a TCP transport with mplex and Yamux.
	let transport = build_transport(&keypair, true)?;

	let behaviour = Behavior::new(BehaviorConfig {
		peer_id: local_peer_id.clone(),
		identify,
		kademlia: KademliaConfig::default(),
		kad_store: MemoryStore::new(local_peer_id.clone()),
	});

	// Create a SwarmBuilder.
	let mut swarm_builder =
		SwarmBuilder::with_tokio_executor(
			transport,
			behaviour,
			local_peer_id.clone(),
		);

	// Build the swarm.
	let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id)
		// .max_negotiating_inbound_streams(SWARM_MAX_NEGOTIATING_INBOUND_STREAMS)
		.build();

	let (to_worker, from_service) = mpsc::channel(8);

	// Initialize the worker.
	let worker = worker::DasNetwork::new(swarm, from_service, metrics);

	// Return the service and worker.
	Ok((service::Service::new(to_worker), worker::DasNetwork::new(swarm, from_service, metrics)))
}

fn build_transport(
	key_pair: &Keypair,
	port_reuse: bool,
) -> Result<transport::Boxed<(PeerId, StreamMuxerBox)>> {
	let noise = NoiseAuthenticated::xx(&key_pair).unwrap();
	let dns_tcp = TokioDnsConfig::system(TokioTcpTransport::new(
		GenTcpConfig::new().nodelay(true).port_reuse(port_reuse),
	))?;

	Ok(dns_tcp
		.upgrade(Version::V1)
		.authenticate(noise)
		.multiplex(YamuxConfig::default())
		.timeout(Duration::from_secs(20))
		.boxed())
}

/// Trait to encapsulate necessary network-related operations.
pub trait NetworkProvider: NetworkDHTProvider + NetworkStateInfo + NetworkSigner {}
impl<T> NetworkProvider for T where T: NetworkDHTProvider + NetworkStateInfo + NetworkSigner {}

// Import core primitives related to sidecars.
pub use melo_core_primitives::{Sidecar, SidecarMetadata, SidecarStatus};
use sp_core::H256;

/// Instantiates a new DHT Worker with the given parameters.
pub fn new_worker(
	from_service: mpsc::Receiver<Command>,
	metrics: Metrics,
	swarm: Swarm<Behavior>,
) -> DasNetwork
where
{
	DasNetwork::new(swarm, from_service, metrics)
}

/// Creates a new channel for communication between the service and worker.
pub fn new_workgroup() -> (mpsc::Sender<Command>, mpsc::Receiver<Command>) {
	mpsc::channel(0)
}

/// Initializes a new Service instance with the specified communication channel.
pub fn new_service(to_worker: mpsc::Sender<Command>) -> Service {
	Service::new(to_worker)
}

/// Conveniently creates both a Worker and Service with the given parameters.
#[allow(clippy::type_complexity)]
pub fn new_worker_and_service(
	from_service: mpsc::Receiver<Command>,
	metrics: Metrics,
	swarm: Swarm<Behavior>,
) -> (DasNetwork, Service) {
	let (to_worker, from_service) = mpsc::channel(0);

	let worker: DasNetwork = new_worker(from_service, metrics, swarm);
	let service = Service::new(to_worker);

	(worker, service)
}

/// Converts a sidecar instance into a Kademlia key.
pub fn sidecar_kademlia_key(sidecar: &Sidecar) -> KademliaKey {
	KademliaKey::from(Vec::from(sidecar.id()))
}

/// Converts a sidecar ID into a Kademlia key.
pub fn kademlia_key_from_sidecar_id(sidecar_id: &H256) -> KademliaKey {
	KademliaKey::from(Vec::from(&sidecar_id[..]))
}
