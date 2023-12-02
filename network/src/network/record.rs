// use std::collections::HashSet;

// use libp2p::{
//     identify::Event as IdentifyEvent,
//     request_response::OutboundRequestId,
//     swarm::{dummy::ConnectionHandler as DummyConnectionHandler, NetworkBehaviour, FromSwarm},
// };
// use void::Void;

// use super::{identity, NetworkId};

// pub struct RecordStore {
//     pub requests: HashSet<(OutboundRequestId, NetworkId, bool)>,
// }

// impl NetworkBehaviour for RecordStore {
//     type ConnectionHandler = DummyConnectionHandler;

//     type ToSwarm = Void;

//     fn handle_established_inbound_connection(
//         &mut self,
//         _connection_id: libp2p::swarm::ConnectionId,
//         peer: libp2p::PeerId,
//         local_addr: &libp2p::Multiaddr,
//         remote_addr: &libp2p::Multiaddr,
//     ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
//         Ok(DummyConnectionHandler)
//     }

//     fn handle_established_outbound_connection(
//         &mut self,
//         _connection_id: libp2p::swarm::ConnectionId,
//         peer: libp2p::PeerId,
//         addr: &libp2p::Multiaddr,
//         role_override: libp2p::core::Endpoint,
//     ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
//         Ok(DummyConnectionHandler)
//     }

//     fn on_swarm_event(&mut self, event: libp2p::swarm::FromSwarm) {
//         match event {
//             FromSwarm::
//         }
//     }

//     fn on_connection_handler_event(
//         &mut self,
//         _peer_id: libp2p::PeerId,
//         _connection_id: libp2p::swarm::ConnectionId,
//         _event: libp2p::swarm::THandlerOutEvent<Self>,
//     ) {
//     }

//     fn poll(
//         &mut self,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<libp2p::swarm::ToSwarm<Self::ToSwarm, libp2p::swarm::THandlerInEvent<Self>>>
//     {
//         todo!()
//     }
// }
