use std::str::FromStr;

use either::Either;
use libp2p::{
    core::ConnectedPoint,
    dcutr::Event as DCUTREvent, //InboundUpgradeError, OutboundUpgradeError},
    identify::Event as IdentifyEvent,
    kad::{Event as KademliaEvent, GetRecordOk, PeerRecord, QueryResult, Record},
    mdns::Event as MdnsEvent,
    multiaddr::Protocol,
    ping::Event as PingEvent,
    relay::{client::Event as RelayClientEvent, Event as RelayServerEvent},
    rendezvous::{self},
    request_response::{
        Event::{
            self as RequestResponseEvent, InboundFailure, Message, OutboundFailure, ResponseSent,
        },
        Message::{Request, Response},
        OutboundRequestId as RequestId, ResponseChannel,
    },
    swarm::{DialError, SwarmEvent}, //ConnectionHandlerUpgrErr
    Multiaddr,
    PeerId,
    Swarm,
    TransportError,
};
use log::{debug, error, info};

use crate::network::{
    behaviour::DecentNetworkBehaviour,
    config::NetworkConfig,
    config::RENDEZVOUS_NAMESPACE,
    protocol::{DecentNetRequest, DecentNetResponse, NetworkNodeRecord},
    utils::get_external_addrs,
    NetworkEvent, NetworkNode,
};

// pub type OutEvent = <<<DecentNetworkBehaviour as NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::OutEvent;
// pub type InEvent = <<<DecentNetworkBehaviour as NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as ProtocolsHandler>::InEvent;
// pub type Handler = <DecentNetworkBehaviour as NetworkBehaviour>::ProtocolsHandler;
// pub type NetworkSwarmEvent = SwarmEvent<OutEvent, Handler>;
// pub type NetworkSwarmEventD = SwarmEvent<NetworkEvent, Handler>;
pub type NetworkSwarmEvent = SwarmEvent<NetworkEvent>;
//     Either<
//         Either<
//             Either<
//                 Either<
//                     Either<
//                         Either<
//                             Either<Either<PingFailure, ConnectionHandlerUpgrErr<Error>>, Error>,
//                             Void,
//                         >,
//                         Error,
//                     >,
//                     ConnectionHandlerUpgrErr<Error>,
//                 >,
//                 Either<
//                     ConnectionHandlerUpgrErr<Either<InHopUpgradeError, OutStopUpgradeError>>,
//                     Void,
//                 >,
//             >,
//             Either<ConnectionHandlerUpgrErr<Either<InStopUpgradeError, OutHopUpgradeError>>, Void>,
//         >,
//         Either<
//             ConnectionHandlerUpgrErr<Either<InboundUpgradeError, OutboundUpgradeError>>,
//             Either<ConnectionHandlerUpgrErr<Error>, Void>,
//         >,
//     >,
// >;

impl DecentNetworkBehaviour {
    pub fn handle_swarm_event(
        swarm: &mut Swarm<DecentNetworkBehaviour>,
        event: NetworkSwarmEvent,
        network_config: &mut NetworkConfig,
    ) {
        match event {
            SwarmEvent::Dialing {
                peer_id: Some(peer_id),
                ..
            } => info!("Dialing Peed Id : {}", peer_id),
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => {
                info!("Listening on : {:?} with {:?}", address, listener_id);
                network_config.listener_ids.push(listener_id);
                if !network_config.relay_dial_mode
                    && address
                        == DecentNetworkBehaviour::get_relayed_addr(
                            network_config,
                            *swarm.local_peer_id(),
                        )
                {
                    error!("Remove Other listening address here");
                    let ids = network_config.listener_ids.clone();

                    let filtered: Vec<_> =
                        ids.into_iter().filter(|id| *id != listener_id).collect();
                    for id in filtered {
                        swarm.remove_listener(id);
                    }
                    let ids = network_config.listener_ids.clone();
                    error!("{:?}", ids);
                }
            }
            SwarmEvent::ConnectionEstablished {
                peer_id: network_id,
                endpoint,
                ..
            } => {
                if !network_config.server_mode {
                    if let Either::Left(behaviour) = &mut swarm.behaviour_mut().rendezvous {
                        behaviour.discover(
                            Some(
                                rendezvous::Namespace::new(RENDEZVOUS_NAMESPACE.to_string())
                                    .unwrap(),
                            ),
                            None,
                            None,
                            network_config.boot_nodes.first().unwrap().network_id,
                        );
                    }
                    if network_config.debug_mode {
                        info!(
                        "Connected to rendezvous point, discovering nodes in '{}' namespace ...",
                        RENDEZVOUS_NAMESPACE
                    );
                    }
                } else {
                    if network_config.debug_mode {
                        info!("Connected to {}", network_id);
                    }
                    if let ConnectedPoint::Listener { send_back_addr, .. } = endpoint {
                        let addr = if network_config.relay_dial_mode && !network_config.server_mode
                        {
                            DecentNetworkBehaviour::get_relayed_addr(network_config, network_id)
                        } else {
                            send_back_addr
                        };
                        let _ = &mut swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&network_id, addr);
                    }
                }
            }
            SwarmEvent::ConnectionClosed {
                peer_id: network_id,
                cause: _,
                ..
            } => {
                if network_config.debug_mode {
                    info!("Disconnected from {}", network_id);
                }
                swarm.behaviour_mut().kademlia.remove_peer(&network_id);
            }
            SwarmEvent::OutgoingConnectionError {
                peer_id: network_id,
                error,
                ..
            } => {
                match network_id {
                    Some(network_id) => {
                        let res = network_config.boot_nodes.first();
                        if let Some(res) = res {
                            if res.network_id == network_id {
                                error!("Failed to connect to boot node: {:?}", error);
                                //swarm.behaviour_mut().kademlia.remove_peer(&network_id);
                            }
                        }
                        if network_config.debug_mode {
                            info!("Failed to connect to {}, error : {}", network_id, error);
                        }
                    }
                    None => {
                        if let DialError::Transport(addrs) = &error {
                            for addr in addrs {
                                let address = &addr.0;
                                if let TransportError::Other(error) = &addr.1 {
                                    if error.kind() == std::io::ErrorKind::Other
                                        && !network_config.server_mode
                                    {
                                        if address
                                            == &(network_config
                                                .boot_nodes
                                                .first()
                                                .unwrap()
                                                .multiaddr)
                                        {
                                            //TODO! : Retry strategy required here.
                                            error!("Boot Server Down : {}", address.to_string());
                                        }
                                        if network_config.debug_mode {
                                            info!("Peer Unavailable : {}", error);
                                        }
                                    }
                                }
                            }
                            if network_config.debug_mode {
                                info!("Failed to connect due to error : {}", error);
                            }
                        }
                        if network_config.debug_mode {
                            info!("Failed to connect due to error : {}", error);
                        }
                    }
                }
            }
            SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
                ..
            } => {
                if network_config.debug_mode {
                    info!(
                        "Incoming connection : {}, local_addr : {} ",
                        send_back_addr, local_addr
                    );
                }
            }
            SwarmEvent::IncomingConnectionError {
                local_addr,
                send_back_addr,
                error,
                ..
            } => {
                error!(
                "Incoming connection error : send_back_address : {}, local_addr : {}, error: {} ",
                send_back_addr, local_addr, error
            );
            }
            SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => {
                error!(
                    "Expired listen address : listener_id : {:?}, address: {} ",
                    listener_id, address
                );
            }
            SwarmEvent::ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => {
                error!(
                    "Listener Closed : listener_id : {:?}, addresses: {:?} , reason : {:?}",
                    listener_id, addresses, reason
                );
            }
            SwarmEvent::ListenerError { listener_id, error } => {
                error!(
                    "Listener Closed : listener_id : {:?}, error: {}",
                    listener_id, error
                );
            }
            // SwarmEvent::BannedPeer { peer_id, endpoint } => {
            //     error!(
            //         "Banned Peer : peer_id : {:?}, endpoint: {:?}",
            //         peer_id, endpoint
            //     );
            // }
            SwarmEvent::Behaviour(_) => {
                if network_config.debug_mode {
                    info!("Behavoiur Event Should be handled by library consumer");
                }
            }
            _ => {}
        }
    }

    pub fn handle_request_msg(
        &mut self,
        requester_id: &PeerId,
        request: DecentNetRequest,
        channel: ResponseChannel<DecentNetResponse>,
        config: &NetworkConfig,
    ) {
        match request {
            DecentNetRequest::Ping => {
                if config.debug_mode {
                    info!("Received Ping, we will send a Pong back.");
                }
                self.protocol
                    .send_response(channel, DecentNetResponse::Pong)
                    .unwrap();
            }
            DecentNetRequest::GetNetworkNodes => {
                let record = self.get_network_record(Some(requester_id), config);
                self.protocol
                    .send_response(channel, DecentNetResponse::Record(record))
                    .unwrap();
            }
            DecentNetRequest::SendNodeRecord(record) => {
                self.load_node_record(&record, config);
                self.protocol
                    .send_response(channel, DecentNetResponse::GotNetworkRecord)
                    .unwrap();
            }
        }
    }

    pub fn handle_response_msg(
        &mut self,
        request_id: RequestId,
        response: DecentNetResponse,
        config: &NetworkConfig,
    ) {
        match response {
            DecentNetResponse::Pong => {
                info!("Received Pong for request {:?}.", request_id);
            }
            DecentNetResponse::Record(record) => {
                if config.debug_mode {
                    info!(
                        "Received Record for request {:?}, records {:?}",
                        request_id, record
                    );
                }
                self.load_node_record(&record, config);
            }
            DecentNetResponse::GotNetworkRecord => {
                if config.debug_mode {
                    info!("Successfully Sent Node Record for to known peers",);
                }
            }
        }
    }

    pub fn handle_req_res_event(
        event: RequestResponseEvent<DecentNetRequest, DecentNetResponse>,
        swarm: &mut Swarm<DecentNetworkBehaviour>,
        config: &NetworkConfig,
    ) {
        match event {
            Message { peer, message } => match message {
                Request {
                    request_id: _,
                    request,
                    channel,
                } => swarm
                    .behaviour_mut()
                    .handle_request_msg(&peer, request, channel, config),
                Response {
                    request_id,
                    response,
                } => swarm
                    .behaviour_mut()
                    .handle_response_msg(request_id, response, config),
            },
            OutboundFailure {
                peer,
                request_id,
                error,
            } => {
                error!(
                    "Outbound Failure for request {:?} to peer: {:?}: {:?}.",
                    request_id, peer, error
                )
            }
            InboundFailure {
                peer,
                request_id,
                error,
            } => error!(
                "Inbound Failure for request {:?} to peer: {:?}: {:?}.",
                request_id, peer, error
            ),
            ResponseSent { peer, request_id } => {
                if config.debug_mode {
                    info!(
                        "ResponseSent for request {:?} to peer: {:?}.",
                        request_id, peer
                    )
                }
            }
        }
    }

    pub fn handle_ping_event(event: PingEvent, config: &NetworkConfig) {
        let PingEvent {
            peer: network_id,
            result,
            ..
        } = event;
        match result {
            Ok(duration) => {
                if !config.server_mode
                    && network_id != (config.boot_nodes).first().unwrap().network_id
                {
                    debug!("Ping to {} is {}ms", network_id, duration.as_millis());
                }
            }
            // Ok(PingSuccess::Ping { rtt }) => {
            //     if !swarm.behaviour_mut().config.server_mode
            //         && network_id
            //             != (swarm.behaviour_mut().config.boot_nodes)
            //                 .first()
            //                 .unwrap()
            //                 .network_id
            //     {
            //         debug!("Ping to {} is {}ms", network_id, rtt.as_millis())
            //     }
            // }
            // Ok(PingSuccess::Pong {}) => {
            //     if !swarm.behaviour_mut().config.server_mode
            //         && network_id
            //             != (swarm.behaviour_mut().config.boot_nodes)
            //                 .first()
            //                 .unwrap()
            //                 .network_id
            //     {
            //         debug!("Pong from {}", network_id)
            //     }
            // }
            Err(err) => {
                error!("Ping to {} failed, cause : {}", network_id, err)
            }
        }
    }

    pub fn handle_rendezvous_event(
        network_event: NetworkEvent,
        swarm: &mut Swarm<DecentNetworkBehaviour>,
        config: &mut NetworkConfig,
    ) {
        match network_event {
            NetworkEvent::RendezvousEvent(Either::Right(
                rendezvous::server::Event::PeerRegistered {
                    peer: network_id,
                    registration,
                },
            )) => {
                info!(
                    "Network Node {} registered for namespace '{}'",
                    network_id, registration.namespace
                );
            }
            NetworkEvent::RendezvousEvent(Either::Right(
                rendezvous::server::Event::DiscoverServed {
                    enquirer: network_id,
                    registrations,
                },
            )) => {
                info!(
                    "Served network node {} with {} registrations",
                    network_id,
                    registrations.len()
                );
            }
            NetworkEvent::RendezvousEvent(Either::Left(
                rendezvous::client::Event::Registered {
                    namespace,
                    ttl,
                    rendezvous_node: network_id,
                },
            )) => {
                info!(
                    "Registered for namespace '{}' at rendezvous point {} for the next {} seconds",
                    namespace, network_id, ttl
                );
            }
            NetworkEvent::RendezvousEvent(Either::Left(
                rendezvous::client::Event::RegisterFailed { error, .. },
            )) => {
                error!("Failed to register {:?}", error);
            }
            NetworkEvent::RendezvousEvent(Either::Left(
                rendezvous::client::Event::Discovered {
                    registrations,
                    cookie: new_cookie,
                    ..
                },
            )) => {
                if !config.server_mode {
                    config.rendezvous_cookie.replace(new_cookie);
                    for registration in registrations {
                        for address in registration.record.addresses() {
                            let network_id = registration.record.peer_id();
                            info!("Discovered Node {} at {}", network_id, address);
                            let p2p_suffix = Protocol::P2p(network_id);
                            let address_with_p2p = if !address
                                .ends_with(&Multiaddr::empty().with(p2p_suffix.clone()))
                            {
                                address.clone().with(p2p_suffix.clone())
                            } else {
                                address.clone()
                            };
                            let conf = config.clone();
                            let addr = if conf.relay_dial_mode {
                                DecentNetworkBehaviour::get_relayed_addr(config, network_id)
                            } else {
                                address_with_p2p
                            };
                            swarm.dial(addr).unwrap()
                        }
                    }
                }
            }

            _ => {}
        }
    }

    pub fn handle_identify_event(
        event: IdentifyEvent,
        swarm: &mut Swarm<DecentNetworkBehaviour>,
        config: &NetworkConfig,
    ) {
        if let IdentifyEvent::Received {
            peer_id: network_id,
            info,
            ..
        } = event
        {
            for addr in info.listen_addrs {
                if config.debug_mode {
                    info!(
                        "Got IdentifyEvent : for Id : {} with Addr : {}",
                        network_id, addr
                    );
                }
                let addr = if !config.server_mode && config.relay_dial_mode {
                    if network_id == config.boot_nodes.first().unwrap().network_id {
                        addr
                    } else {
                        DecentNetworkBehaviour::get_relayed_addr(config, network_id)
                    }
                } else {
                    addr
                };
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&network_id, addr);
                // let already_sent = swarm
                //     .behaviour_mut()
                //     .requests
                //     .iter()
                //     .any(|(_, id, _)| *id == network_id);
                // if !already_sent {
                //     let record = swarm
                //         .behaviour_mut()
                //         .get_network_record(Some(&network_id.clone()), config);
                //     for node in &record.nodes {
                //         let network_id = node.network_id.parse::<PeerId>().unwrap();
                //         let record = record.clone();
                //         let node = record
                //             .nodes
                //             .iter()
                //             .find(|n| n.network_id != network_id.to_base58());
                //         if let Some(node) = node {
                //             let req_id = swarm.behaviour_mut().protocol.send_request(
                //                 &network_id,
                //                 DecentNetRequest::SendNodeRecord(NetworkNodeRecord {
                //                     nodes: vec![node.clone()],
                //                 }),
                //             );
                //             swarm
                //                 .behaviour_mut()
                //                 .requests
                //                 .insert((req_id, network_id, true));
                //         }
                //     }
                // }
            }
            if !config.server_mode {
                let network_id = (config.boot_nodes).first().unwrap().network_id;
                if let Either::Left(behaviour) = &mut swarm.behaviour_mut().rendezvous {
                    behaviour
                        .register(
                            rendezvous::Namespace::from_static(RENDEZVOUS_NAMESPACE),
                            network_id,
                            None,
                        )
                        .unwrap();
                }
            }
        }
    }

    pub fn handle_kademlia_event(event: KademliaEvent) {
        match event {
            KademliaEvent::OutboundQueryProgressed { result, .. } => {
                match result {
                    QueryResult::GetRecord(Ok(GetRecordOk::FoundRecord(PeerRecord {
                        record: Record { key, value, .. },
                        ..
                    }))) => {
                        // for PeerRecord {
                        //     record: Record { key, value, .. },
                        //     ..
                        // } in ok
                        // {
                        info!(
                            "Got record {:?} {:?}.",
                            std::str::from_utf8(key.as_ref()).unwrap(),
                            std::str::from_utf8(&value).unwrap(),
                        );
                        // let mut unique_nodes = HashSet::new();
                        // for node in nodes {
                        //     unique_nodes.insert(node);
                        // }
                        // unique_nodes.iter().for_each(|p| info!("{}", p));
                        // }
                    }
                    QueryResult::GetRecord(Err(err)) => {
                        error!("Failed to get record: {:?}.", err);
                    }
                    QueryResult::Bootstrap(Ok(ok)) => {
                        info!("Bootstrap completed > Found Node : {}", ok.peer);
                        // let addrs = swarm
                        //     .behaviour_mut()
                        //     .kademlia
                        //     .addresses_of_peer(&ok.peer.clone());
                        // debug!("{:?}", addrs);
                    }
                    QueryResult::GetClosestPeers(result) => {
                        if let Ok(peers) = result {
                            info!("Found {:?} closest peers", peers.key);
                            for peer in peers.peers {
                                info!("{}", peer);
                            }
                        }
                    }
                    other => {
                        debug!("{:?}", other);
                    }
                }
            }
            KademliaEvent::InboundRequest { request, .. } => {
                debug!("Inbound Request: {:?}", request);
            }
            KademliaEvent::RoutablePeer { peer, address } => {
                debug!("RoutablePeer: {}, addr : {}", peer, address);
            }
            KademliaEvent::UnroutablePeer { peer } => {
                debug!("UnroutablePeer: {}", peer);
            }
            KademliaEvent::PendingRoutablePeer { peer, address } => {
                debug!("PendingRoutablePeer: {}, addr : {}", peer, address);
            }
            KademliaEvent::RoutingUpdated {
                peer,
                addresses,
                old_peer,
                bucket_range,
                is_new_peer,
            } => {
                debug!(
                    "RoutingUpdated: {:?} {:?} {:?} {:?} {:?}",
                    peer, addresses, old_peer, bucket_range, is_new_peer
                );
            }
            KademliaEvent::ModeChanged { new_mode } => {
                debug!("ModeChanged: {:?}", new_mode);
            }
        }
    }

    pub fn handle_mdns_event(
        event: MdnsEvent,
        swarm: &mut Swarm<DecentNetworkBehaviour>,
        config: &NetworkConfig,
    ) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (network_id, addr) in discovered_list {
                    if config.local_discovery {
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .add_node_to_partial_view(network_id);
                        swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&network_id, addr);
                    }
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (network_id, _addr) in expired_list {
                    if !swarm
                        .behaviour()
                        .mdns
                        .as_ref()
                        .unwrap()
                        .discovered_nodes()
                        .any(|id| id == &network_id)
                    {
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .remove_node_from_partial_view(&network_id);
                    }
                }
            }
        }
    }

    pub fn handle_relay_server_event(event: RelayServerEvent) {
        match event {
            RelayServerEvent::ReservationReqAccepted {
                src_peer_id,
                renewed,
            } => {
                info!(
                    "ReservationReqAccepted: {:?}, renewed : {}",
                    src_peer_id, renewed
                );
            }
            // RelayServerEvent::ReservationReqAcceptFailed { src_peer_id, error } => {
            //     error!("ReservationReqAcceptFailed: {}, {}", src_peer_id, error);
            // }
            RelayServerEvent::ReservationReqDenied { src_peer_id } => {
                error!("ReservationReqDenied: {}", src_peer_id);
            }
            // RelayServerEvent::ReservationReqDenyFailed { src_peer_id, error } => {
            //     debug!("ReservationReqTimeout: {}, Error : {}", src_peer_id, error);
            // }
            RelayServerEvent::ReservationTimedOut { src_peer_id } => {
                debug!("ReservationReqTimeout: {}", src_peer_id);
            }
            RelayServerEvent::CircuitReqDenied {
                src_peer_id,
                dst_peer_id,
            } => {
                error!("CircuitReqDenied: src={}, dst={}", src_peer_id, dst_peer_id);
            }
            // RelayServerEvent::CircuitReqOutboundConnectFailed {
            //     src_peer_id,
            //     dst_peer_id,
            //     error,
            // } => {
            //     error!(
            //         "CircuitReqOutboundConnectFailed: src={}, dst={}Error : {}",
            //         src_peer_id, dst_peer_id, error
            //     );
            // }
            // RelayServerEvent::CircuitReqReceiveFailed { src_peer_id, error } => {
            //     error!(
            //         "CircuitReqReceiveFailed: src={}, Error : {}",
            //         src_peer_id, error
            //     );
            // }
            // RelayServerEvent::CircuitReqDenyFailed {
            //     src_peer_id,
            //     dst_peer_id,
            //     error,
            // } => {
            //     error!(
            //         "CircuitReqDenyFailed: src={}, dst={}, Error : {}",
            //         src_peer_id, dst_peer_id, error
            //     );
            // }
            RelayServerEvent::CircuitReqAccepted {
                src_peer_id,
                dst_peer_id,
            } => {
                info!(
                    "CircuitReqAccepted: src={}, dst={}",
                    src_peer_id, dst_peer_id,
                );
            }
            // RelayServerEvent::CircuitReqAcceptFailed {
            //     src_peer_id,
            //     dst_peer_id,
            //     error,
            // } => {
            //     error!(
            //         "CircuitReqAcceptFailed: src={}, dst={}, Error : {}",
            //         src_peer_id, dst_peer_id, error
            //     );
            // }
            RelayServerEvent::CircuitClosed {
                src_peer_id,
                dst_peer_id,
                error,
            } => {
                debug!(
                    "CircuitClosed: src={}, dst={}, error {}",
                    src_peer_id,
                    dst_peer_id,
                    error.unwrap()
                );
            }
            _ => {}
        }
    }

    pub fn handle_relay_client_event(event: RelayClientEvent) {
        match event {
            RelayClientEvent::ReservationReqAccepted {
                relay_peer_id,
                limit,
                renewal,
            } => {
                info!(
                    "ReservationReqAccepted: {}, renewed : {}, limit : {:?}",
                    relay_peer_id, renewal, limit
                );
            }
            // RelayClientEvent::ReservationReqFailed {
            //     relay_peer_id,
            //     renewal,
            //     error,
            // } => {
            //     info!(
            //         "ReservationReqFailed: {}, renewed : {} error: {}",
            //         relay_peer_id, renewal, error
            //     );
            // }
            // RelayClientEvent::OutboundCircuitReqFailed {
            //     relay_peer_id,
            //     error,
            // } => {
            //     info!(
            //         "OutboundCircuitReqFailed: {} error : {}",
            //         relay_peer_id, error
            //     );
            // }
            // RelayClientEvent::InboundCircuitReqDenied { src_peer_id } => {
            //     info!("InboundCircuitReqDenied: {}", src_peer_id);
            // }
            // RelayClientEvent::InboundCircuitReqFailed {
            //     relay_peer_id,
            //     error,
            // } => {
            //     info!(
            //         "InboundCircuitReqFailed: {}, error : {}",
            //         relay_peer_id, error
            //     );
            // }
            // RelayClientEvent::InboundCircuitReqDenyFailed { src_peer_id, error } => {
            //     info!(
            //         "ReservationReqAccepted: {}, renewed : {}",
            //         src_peer_id, error
            //     );
            // }
            RelayClientEvent::InboundCircuitEstablished { src_peer_id, limit } => {
                info!(
                    "InboundCircuitEstablished: src={}, limit={:?}",
                    src_peer_id, limit
                );
            }
            RelayClientEvent::OutboundCircuitEstablished {
                relay_peer_id,
                limit,
            } => {
                info!(
                    "InboundCircuitEstablished: src={}, limit={:?}",
                    relay_peer_id, limit
                );
            }
        }
    }

    pub fn handle_dcutr_event(event: DCUTREvent) {
        let DCUTREvent { remote_peer_id, .. } = event;
        info!("DirectConnectionUpgradeSuccess: {}", remote_peer_id);

        // match event {
        //     DCUTREvent::InitiatedDirectConnectionUpgrade {
        //         remote_peer_id,
        //         local_relayed_addr,
        //     } => {
        //         info!(
        //             "InitiateDirectConnectionUpgrade: {}, {}",
        //             remote_peer_id, local_relayed_addr
        //         );
        //     }
        //     DCUTREvent::RemoteInitiatedDirectConnectionUpgrade {
        //         remote_peer_id,
        //         remote_relayed_addr,
        //     } => {
        //         info!(
        //             "RemoteInitiatedDirectConnectionUpgrade: {}, {}",
        //             remote_peer_id, remote_relayed_addr
        //         );
        //     }
        //     DCUTREvent::DirectConnectionUpgradeSucceeded { remote_peer_id } => {
        //         info!("DirectConnectionUpgradeSuccess: {}", remote_peer_id);
        //     }
        //     DCUTREvent::DirectConnectionUpgradeFailed {
        //         remote_peer_id,
        //         error,
        //     } => {
        //         error!(
        //             "DirectConnectionUpgradeFailed: {}, error : {}",
        //             remote_peer_id, error
        //         );
        //     }
        //     _ => {}
        // }
    }

    pub fn handle_network_event(
        network_event: NetworkEvent,
        network_config: &mut NetworkConfig,
        swarm: &mut Swarm<DecentNetworkBehaviour>,
    ) {
        // let cookie = &mut network_config.rendezvous_cookie;
        match network_event {
            NetworkEvent::Ping(event) => Self::handle_ping_event(event, network_config),
            NetworkEvent::Floodsub(event) => {
                debug!("Floodsub event: {:?}, Should be handled by lib", event);
            }
            NetworkEvent::RendezvousEvent(_) => {
                Self::handle_rendezvous_event(network_event, swarm, network_config)
            }
            NetworkEvent::Kademlia(event) => Self::handle_kademlia_event(event),
            NetworkEvent::Mdns(event) => Self::handle_mdns_event(event, swarm, network_config),
            NetworkEvent::Identify(event) => {
                Self::handle_identify_event(event, swarm, network_config)
            }
            NetworkEvent::RequestResponse(event) => {
                Self::handle_req_res_event(event, swarm, network_config)
            }
            // NetworkEvent::RelayServer(event) => Self::handle_relay_server_event(event),
            // NetworkEvent::RelayClient(event) => Self::handle_relay_client_event(event),
            NetworkEvent::RelayEvent(event) => Self::handle_relay_server_event(event),
            NetworkEvent::DCUtR(event) => Self::handle_dcutr_event(event),
        }
    }

    pub fn get_network_record(
        &mut self,
        requester_id: Option<&PeerId>,
        config: &NetworkConfig,
    ) -> NetworkNodeRecord {
        let boot_node_ids = config
            .boot_nodes
            .clone()
            .iter()
            .map(|boot_node| boot_node.network_id.to_base58())
            .collect::<Vec<_>>();
        let mut unique_nodes = vec![];
        let nodes = self.kademlia.kbuckets();
        nodes.for_each(|bucketref| {
            bucketref
                .iter()
                .filter(|refview| {
                    let requester_id = requester_id.map(|id| id.to_base58());
                    let network_id = refview.node.key.preimage();
                    let network_id_str = network_id.to_base58();
                    if !boot_node_ids.contains(&network_id_str) {
                        if requester_id.is_none() {
                            panic!("Requester id is None");
                        }
                        requester_id.unwrap() != network_id_str
                    } else {
                        false
                    }
                })
                .for_each(|refview| {
                    let network_id = refview.node.key.preimage();
                    let addr = &(refview.node.value).clone().into_vec();
                    let addrs = get_external_addrs(addr)
                        .into_iter()
                        .map(|addr| addr.to_string())
                        .collect();
                    unique_nodes.push(NetworkNode {
                        network_id: network_id.to_base58(),
                        multiaddr: addrs,
                    });
                });
        });
        NetworkNodeRecord {
            nodes: unique_nodes,
        }
    }

    pub fn load_node_record(&mut self, record: &NetworkNodeRecord, config: &NetworkConfig) {
        let nodes = record.nodes.iter().map(|node| {
            let network_id = PeerId::from_str(node.network_id.as_str()).unwrap();
            let multiaddrs = node
                .multiaddr
                .iter()
                .map(|addr| Multiaddr::from_str(addr).unwrap())
                .collect::<Vec<_>>();
            (network_id, multiaddrs)
        });
        for (network_id, multiaddrs) in nodes {
            for multiaddr in multiaddrs {
                if config.relay_dial_mode {
                    let addr = DecentNetworkBehaviour::get_relayed_addr(config, network_id);
                    self.kademlia.add_address(&network_id, addr);
                } else {
                    self.kademlia.add_address(&network_id, multiaddr.clone());
                }
                //TODO!
                self.protocol
                    .send_request(&network_id, DecentNetRequest::Ping);
            }
        }
    }

    pub fn get_relayed_addr(config: &NetworkConfig, local_id: PeerId) -> Multiaddr {
        let node = config.boot_nodes.first().unwrap();
        let relay_id = Protocol::P2p(node.network_id);
        let addr = node
            .multiaddr
            .clone()
            .with(relay_id)
            .with(Protocol::P2pCircuit);
        addr.with(Protocol::P2p(local_id))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use libp2p::PeerId;

    use crate::network::{behaviour::DecentNetworkBehaviour, config::NetworkConfig, BootNode};

    #[test]
    fn test_relayed_addr() {
        let addr_str = "/ip4/127.0.0.1/tcp/26117";
        let relay_id = "12D3KooWPjceQrSwdWXPyLLeABRXmuqt69Rg3sBYbU1Nft9HyQ6X";
        let local_id = "12D3KooWH3uVF6wv47WnArKHk5p6cvgCJEb74UTmxztmQDc298L3";
        let net_con = NetworkConfig {
            boot_nodes: vec![BootNode {
                network_id: relay_id.parse().unwrap(),
                multiaddr: addr_str.parse().unwrap(),
            }],
            ..Default::default()
        };
        let local_id = PeerId::from_str(local_id).unwrap();
        let addr = DecentNetworkBehaviour::get_relayed_addr(&net_con, local_id);
        assert_eq!(
            addr.to_string(),
            format!("{}/p2p/{}/p2p-circuit/p2p/{}", addr_str, relay_id, local_id)
        );
    }
}
