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
	PeerId,
	core::{
		muxing::StreamMuxerBox,
		transport::{self},
		upgrade::Version,
	},
	dns::TokioDnsConfig,
	identify::Config as IdentifyConfig,
	identity,
	identity::Keypair,
	kad::{store::MemoryStore, KademliaConfig},
	noise::NoiseAuthenticated,
	swarm::SwarmBuilder,
	tcp::{tokio::Transport as TokioTcpTransport, Config as GenTcpConfig},
	yamux::YamuxConfig,
	Transport,
};
use melo_core_primitives::config;

pub use log::warn;

pub use node_primitives::AccountId;
pub use sc_client_api::Backend;
pub use sc_network::{DhtEvent, KademliaKey, NetworkDHTProvider, NetworkSigner, NetworkStateInfo};
pub use sp_runtime::traits::{Block, Header};
pub use std::sync::Arc;
use std::time::Duration;

pub use behaviour::{Behavior, BehaviorConfig, BehaviourEvent};
pub use service::{DasNetworkConfig, Service};
pub use shared::Command;
pub use worker::DasNetwork;

mod behaviour;
mod service;
mod shared;
mod worker;

const SWARM_MAX_NEGOTIATING_INBOUND_STREAMS: usize = 5000;

/// Creates a new [`DasNetwork`] instance.
/// The [`DasNetwork`] instance is composed of a [`Service`] and a [`Worker`].
pub fn create(
	keypair: identity::Keypair,
	protocol_version: String,
	prometheus_registry: Option<prometheus_endpoint::Registry>,
	config: DasNetworkConfig,
) -> Result<(service::Service, worker::DasNetwork)> {
	let local_peer_id = PeerId::from(keypair.public());

	let protocol_version = format!("/melodot-das/{}", protocol_version);
	let identify = IdentifyConfig::new(protocol_version.clone(), keypair.public());

	let transport = build_transport(&keypair, true)?;

	let behaviour = Behavior::new(BehaviorConfig {
		peer_id: local_peer_id,
		identify,
		kademlia: KademliaConfig::default(),
		kad_store: MemoryStore::new(local_peer_id),
	})?;

	let swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id)
		.max_negotiating_inbound_streams(SWARM_MAX_NEGOTIATING_INBOUND_STREAMS)
		.build();

	let (to_worker, from_service) = mpsc::channel(8);

	Ok((
		service::Service::new(to_worker, config.parallel_limit),
		worker::DasNetwork::new(swarm, from_service, prometheus_registry, &config),
	))
}

/// Creates a new [`DasNetwork`] instance with default configuration.
pub fn default(
	config: Option<DasNetworkConfig>,
	keypair: Option<identity::Keypair>,
) -> Result<(service::Service, worker::DasNetwork)> {
	let keypair = match keypair {
		Some(keypair) => keypair,
		None => {
			identity::Keypair::generate_ed25519()
		}
	};

	let config = match config {
		Some(config) => config,
		None => DasNetworkConfig::default(),
	};

	let metric_registry = prometheus_endpoint::Registry::default();

	create(
		keypair,
		config::DAS_NETWORK_VERSION.to_string(),
		Some(metric_registry),
		config,
	)
}

fn build_transport(
	key_pair: &Keypair,
	port_reuse: bool,
) -> Result<transport::Boxed<(PeerId, StreamMuxerBox)>> {
	let noise = NoiseAuthenticated::xx(key_pair).unwrap();
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
