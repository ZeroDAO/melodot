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
use crate::{Behavior, BehaviourEvent, Command, DasNetworkConfig};
use futures::{
	channel::{mpsc, oneshot},
	stream::StreamExt,
};
use libp2p::{
	identify::Event as IdentifyEvent,
	kad::{
		BootstrapOk, GetRecordOk, InboundRequest, KademliaEvent, PutRecordOk, QueryId, QueryResult,
		Record,
	},
	metrics::Metrics,
	multiaddr::Protocol,
	swarm::{ConnectionError, Swarm, SwarmEvent},
	PeerId, Multiaddr,
};
use log::{debug, info, trace, warn, error};
use std::{collections::HashMap, fmt::Debug};

const MAX_RETRIES: u8 = 3;

enum QueryResultSender {
	PutRecord(oneshot::Sender<Result<(), anyhow::Error>>),
	GetRecord(oneshot::Sender<Result<Vec<Record>, anyhow::Error>>),
	Bootstrap(oneshot::Sender<Result<(), anyhow::Error>>),
}

macro_rules! handle_send {
	($sender_variant:ident, $msg:expr, $result:expr) => {
		if let Some(QueryResultSender::$sender_variant(ch)) = $msg {
			if ch.send($result).is_err() {
				debug!("Failed to send result");
			}
		}
	};
}

pub struct DasNetwork {
	swarm: Swarm<Behavior>,
	command_receiver: mpsc::Receiver<Command>,
	output_senders: Vec<mpsc::Sender<BehaviourEvent>>,
	query_id_receivers: HashMap<QueryId, QueryResultSender>,
	pending_routing: HashMap<PeerId, QueryResultSender>,
	retry_counts: HashMap<PeerId, u8>,
	metrics: Metrics,
	known_addresses: HashMap<PeerId, Vec<String>>,
}

impl DasNetwork {
	pub fn new(
		swarm: Swarm<Behavior>,
		command_receiver: mpsc::Receiver<Command>,
		metrics: Metrics,
		config: &DasNetworkConfig,
	) -> Self {
		let mut swarm = swarm;
		let mut known_addresses = HashMap::new();

		// Add bootstrap node addresses to the swarm and known_addresses
		for addr in &config.bootstrap_nodes {
			if let Ok(multiaddr) = addr.parse::<Multiaddr>() {
				if let Some(peer_id) = multiaddr.iter().find_map(|p| {
					if let Protocol::P2p(hash) = p {
						PeerId::from_multihash(hash).ok()
					} else {
						None
					}
				}) {
					swarm.behaviour_mut().kademlia.add_address(&peer_id, multiaddr.clone());
					known_addresses.entry(peer_id).or_insert_with(Vec::new).push(addr.clone());
				} else {
					warn!("Bootstrap node address does not contain a Peer ID: {}", addr);
				}
			} else {
				warn!("Invalid multiaddr for bootstrap node: {}", addr);
			}
		}

		// Start listening on the specified address and port from config
		let listen_addr = format!("/ip4/{}/tcp/{}", config.listen_addr, config.listen_port);
		if let Err(e) = Swarm::listen_on(&mut swarm, listen_addr.parse().unwrap()) {
			error!("Error starting to listen on {}: {}", listen_addr, e);
		}

		Self {
			swarm,
			command_receiver,
			output_senders: Vec::new(),
			query_id_receivers: HashMap::default(),
			pending_routing: HashMap::default(),
			retry_counts: HashMap::default(),
			metrics,
			known_addresses,
		}
	}

	pub async fn run(mut self) {
		if !self.known_addresses.is_empty() {
			for (peer_id, addrs) in self.known_addresses.iter() {
				for addr in addrs {
					if let Ok(multiaddr) = addr.parse() {
						self.swarm.behaviour_mut().kademlia.add_address(peer_id, multiaddr);
					}
				}
			}

			match self.swarm.behaviour_mut().kademlia.bootstrap() {
				Ok(_) => info!("Bootstrap initiated."),
				Err(e) => warn!("Bootstrap failed to start: {:?}", e),
			}
		}

		loop {
			tokio::select! {
				swarm_event = self.swarm.select_next_some() => {
					self.handle_swarm_event(swarm_event).await;
				},
				command = self.command_receiver.select_next_some() => {
					self.handle_command(command).await;
				}
			}
		}
	}

	async fn handle_swarm_event<E: Debug>(&mut self, event: SwarmEvent<BehaviourEvent, E>) {
		match event {
			SwarmEvent::Behaviour(BehaviourEvent::Kademlia(event)) =>
				self.handle_kademlia_event(event).await,
			SwarmEvent::Behaviour(BehaviourEvent::Identify(event)) =>
				self.handle_identify_event(event).await,
			SwarmEvent::NewListenAddr { address, .. } => {
				let peer_id = self.swarm.local_peer_id();
				let address_with_peer = address.with(Protocol::P2p(peer_id.clone().into()));
				info!("Local node is listening on {:?}", address_with_peer);
			},
			SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
				debug!("Connection closed with peer {:?}", peer_id);
				if let Some(cause) = cause {
					match cause {
						ConnectionError::IO(_) | ConnectionError::Handler(_) => {
							// Retry connection with peer
							let should_remove = {
								let retry_count =
									self.retry_counts.entry(peer_id.clone()).or_insert(0);
								if *retry_count < MAX_RETRIES {
									*retry_count += 1;
									debug!(
										"Will retry connection with peer {:?} (attempt {})",
										peer_id, *retry_count
									);
									// Optionally: Add logic to delay the next connection attempt
									false
								} else {
									debug!(
										"Removed peer {:?} after {} failed attempts",
										peer_id, *retry_count
									);
									true
								}
							};

							if should_remove {
								self.swarm.behaviour_mut().kademlia.remove_peer(&peer_id);
								self.retry_counts.remove(&peer_id);
							}
						},
						_ => {},
					}
				}
			},
			SwarmEvent::Dialing(peer_id) => debug!("Dialing {}", peer_id),
			_ => trace!("Unhandled Swarm event: {:?}", event),
		}
	}

	async fn handle_kademlia_event(&mut self, event: KademliaEvent) {
		trace!("Kademlia event: {:?}", event);
		match event {
			KademliaEvent::RoutingUpdated { peer, is_new_peer, addresses, old_peer, .. } => {
				debug!(
					"Updated routing information. Affected Peer: {:?}. New Peer?: {:?}. Associated Addresses: {:?}. Previous Peer (if replaced): {:?}",
					peer, is_new_peer, addresses, old_peer
				);
				let msg = self.pending_routing.remove(&peer.into());
				handle_send!(Bootstrap, msg, Ok(()));
			},
			KademliaEvent::RoutablePeer { peer, address } => {
				debug!(
					"Identified a routable peer. Peer ID: {:?}. Associated Address: {:?}",
					peer, address
				);
			},
			KademliaEvent::UnroutablePeer { peer } => {
				debug!("Identified an unroutable peer. Peer ID: {:?}", peer);
			},
			KademliaEvent::PendingRoutablePeer { peer, address } => {
				debug!("Identified a peer pending to be routable. Peer ID: {:?}. Tentative Address: {:?}", peer, address);
			},
			KademliaEvent::InboundRequest { request } => {
				trace!("Received an inbound request: {:?}", request);
				if let InboundRequest::PutRecord { source, record, .. } = request {
					if let Some(block_ref) = record {
						trace!(
							"Received an inbound PUT request. Record Key: {:?}. Request Source: {:?}",
							block_ref.key, source
						);
					}
				}
			},
			KademliaEvent::OutboundQueryProgressed { id, result, .. } => match result {
				QueryResult::GetRecord(result) => {
					let msg = self.query_id_receivers.remove(&id);
					match result {
						Ok(GetRecordOk::FoundRecord(rec)) =>
							handle_send!(GetRecord, msg, Ok(vec![rec.record])),
						Ok(GetRecordOk::FinishedWithNoAdditionalRecord { .. }) =>
							handle_send!(GetRecord, msg, Err(anyhow::anyhow!("No record found."))),
						Err(err) => handle_send!(GetRecord, msg, Err(err.into())),
					}
				},
				QueryResult::PutRecord(result) => {
					let msg = self.query_id_receivers.remove(&id);
					match result {
						Ok(PutRecordOk { .. }) => handle_send!(PutRecord, msg, Ok(())),
						Err(err) => handle_send!(PutRecord, msg, Err(err.into())),
					}
				},
				QueryResult::Bootstrap(result) => match result {
					Ok(BootstrapOk { peer, num_remaining }) => {
						trace!("BootstrapOK event. PeerID: {peer:?}. Num remaining: {num_remaining:?}.");
						if num_remaining == 0 {
							let msg = self.query_id_receivers.remove(&id);
							handle_send!(Bootstrap, msg, Ok(()));
						}
					},
					Err(err) => {
						trace!("Bootstrap error event. Error: {err:?}.");
						let msg = self.query_id_receivers.remove(&id);
						handle_send!(Bootstrap, msg, Err(err.into()));
					},
				},
				_ => {},
			},
		}
	}

	async fn handle_identify_event(&mut self, event: IdentifyEvent) {
		if let IdentifyEvent::Received { peer_id, info } = event {
			debug!(
				"IdentifyEvent::Received; peer_id={:?}, protocols={:?}",
				peer_id, info.protocols
			);

			for addr in info.listen_addrs {
				self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
			}
		}
	}

	async fn handle_command(&mut self, command: Command) {
		match command {
			Command::StartListening { addr, sender } =>
				_ = match self.swarm.listen_on(addr) {
					Ok(_) => sender.send(Ok(())),
					Err(e) => sender.send(Err(e.into())),
				},
			Command::AddAddress { peer_id, peer_addr, sender } => {
				self.swarm.behaviour_mut().kademlia.add_address(&peer_id, peer_addr.clone());
				self.pending_routing.insert(peer_id, QueryResultSender::Bootstrap(sender));
			},
			Command::Stream { sender } => {
				self.output_senders.push(sender);
			},
			Command::Bootstrap { sender } => {
				if let Ok(query_id) = self.swarm.behaviour_mut().kademlia.bootstrap() {
					self.query_id_receivers.insert(query_id, QueryResultSender::Bootstrap(sender));
				} else {
					warn!("DHT is empty, unable to bootstrap.");
				}
			},
			Command::GetKadRecord { key, sender } => {
				let query_id = self.swarm.behaviour_mut().kademlia.get_record(key);
				self.query_id_receivers.insert(query_id, QueryResultSender::GetRecord(sender));
			},
			Command::PutKadRecord { record, quorum, sender } => {
				if let Ok(query_id) = self.swarm.behaviour_mut().kademlia.put_record(record, quorum)
				{
					self.query_id_receivers.insert(query_id, QueryResultSender::PutRecord(sender));
				} else {
					warn!("Failed to execute put_record.");
				}
			},
			Command::RemoveRecords { keys, sender } => {
				for key in keys {
					self.swarm.behaviour_mut().kademlia.remove_record(&key);
				}
				sender.send(Ok(())).unwrap_or_else(|_| {
					debug!("Failed to send result");
				});
			},
		}
	}
}
