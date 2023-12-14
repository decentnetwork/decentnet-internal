use either::Either;
use libp2p::{
    dcutr::Event as DCUtREvent,
    floodsub::FloodsubEvent,
    identify::Event as IdentifyEvent,
    identity::ed25519,
    kad::Event as KademliaEvent,
    mdns::Event as MdnsEvent,
    ping::Event as PingEvent,
    relay::Event as RelayServerEvent,
    rendezvous::{
        client::{Behaviour as RendezvousClientBehaviour, Event as RendezvousClientEvent},
        server::{Behaviour as RendezvousServerBehaviour, Event as RendezvousServerEvent},
    },
    request_response::Event as RequestResponseEvent,
    Multiaddr, PeerId,
};
use rkyv::{Archive, Deserialize, Serialize};

use self::protocol::{DecentNetRequest, DecentNetResponse};

pub mod behaviour;
pub mod config;
pub mod handler;
pub mod identity;
pub mod protocol;
pub mod record;
pub mod utils;

pub type NetworkId = PeerId;

pub trait IdentityImpl<'a> {
    type PublicKey;

    fn id(&self) -> NetworkId;
    fn public_key(&self) -> Self::PublicKey;

    fn gen_random_id() -> Self;

    //This is the only case we provide private key to the api,
    //this will avoids stealing key from unintended memory access by extension or malware;
    fn gen_random_id_with_private() -> (ed25519::SecretKey, Self);

    fn from_bytes(bytes: impl AsMut<[u8]>) -> Self;

    // fn auth_key_pair(&self) -> Result<AuthenticKeypair<X25519Spec>, NoiseError>;
}

#[derive(Clone)]
pub struct BootNode {
    pub network_id: NetworkId,
    pub multiaddr: Multiaddr,
}

#[derive(Clone, Debug, Archive, Deserialize, Serialize)]
pub enum ClientRelayMode {
    Disabled,
    Listener,
    Dialer,
}

#[derive(Clone, Debug, Archive, Deserialize, Serialize)]
pub struct NetworkNode {
    pub network_id: String,
    pub multiaddr: Vec<String>,
}

pub enum RendezvousBehaviour {
    Client(RendezvousClientBehaviour),
    Server(RendezvousServerBehaviour),
}

#[derive(Debug)]
pub enum NetworkEvent {
    Ping(PingEvent),
    Floodsub(FloodsubEvent),
    Kademlia(KademliaEvent),
    Mdns(MdnsEvent),
    Identify(IdentifyEvent),
    // RelayEvent(Either<RelayClientEvent, RelayServerEvent>),
    RelayEvent(RelayServerEvent),
    DCUtR(DCUtREvent),
    RequestResponse(RequestResponseEvent<DecentNetRequest, DecentNetResponse>),
    RendezvousEvent(Either<RendezvousClientEvent, RendezvousServerEvent>),
}

impl From<PingEvent> for NetworkEvent {
    fn from(event: PingEvent) -> Self {
        NetworkEvent::Ping(event)
    }
}

impl From<FloodsubEvent> for NetworkEvent {
    fn from(event: FloodsubEvent) -> Self {
        NetworkEvent::Floodsub(event)
    }
}

impl From<KademliaEvent> for NetworkEvent {
    fn from(event: KademliaEvent) -> Self {
        NetworkEvent::Kademlia(event)
    }
}

impl From<MdnsEvent> for NetworkEvent {
    fn from(event: MdnsEvent) -> Self {
        NetworkEvent::Mdns(event)
    }
}

// impl From<RelayServerEvent> for NetworkEvent {
//     fn from(event: RelayServerEvent) -> Self {
//         NetworkEvent::RelayServer(event)
//     }
// }

// impl From<RelayClientEvent> for NetworkEvent {
//     fn from(event: RelayClientEvent) -> Self {
//         NetworkEvent::RelayClient(event)
//     }
// }

impl From<DCUtREvent> for NetworkEvent {
    fn from(event: DCUtREvent) -> Self {
        NetworkEvent::DCUtR(event)
    }
}

impl From<RelayServerEvent> for NetworkEvent {
    fn from(event: RelayServerEvent) -> Self {
        NetworkEvent::RelayEvent(event)
    }
}

impl From<RequestResponseEvent<DecentNetRequest, DecentNetResponse>> for NetworkEvent {
    fn from(event: RequestResponseEvent<DecentNetRequest, DecentNetResponse>) -> Self {
        NetworkEvent::RequestResponse(event)
    }
}

impl From<IdentifyEvent> for NetworkEvent {
    fn from(event: IdentifyEvent) -> Self {
        NetworkEvent::Identify(event)
    }
}

// impl From<Either<RelayClientEvent, RelayServerEvent>> for NetworkEvent {
//     fn from(event: Either<RelayClientEvent, RelayServerEvent>) -> Self {
//         match event {
//             Either::Left(event) => NetworkEvent::RelayEvent(Either::Left(event)),
//             Either::Right(event) => NetworkEvent::RelayEvent(Either::Right(event)),
//         }
//     }
// }

impl From<Either<RendezvousClientEvent, RendezvousServerEvent>> for NetworkEvent {
    fn from(event: Either<RendezvousClientEvent, RendezvousServerEvent>) -> Self {
        NetworkEvent::RendezvousEvent(event)
    }
}
