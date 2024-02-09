use either::Either;
use libp2p::{
    core::transport::ListenerId,
    dcutr::Behaviour as DCUtR,
    floodsub::Floodsub,
    identify::{Behaviour as Identify, Config as IdentifyConfig},
    kad::{store::MemoryStore, Behaviour as Kademlia},
    mdns::tokio::Behaviour as Mdns,
    multiaddr::Protocol,
    noise::Config as NoiseConfig,
    ping::{Behaviour as Ping, Config as PingConfig},
    relay::{client::Behaviour as RelayClient, Behaviour as RelayServer, Config as RelayConfig},
    rendezvous::{
        client::Behaviour as RendezvousClientBehaviour,
        server::Behaviour as RendezvousServerBehaviour,
    },
    request_response::{Behaviour as RequestResponse, Config as RequestResponseConfig},
    swarm::{behaviour::toggle::Toggle, NetworkBehaviour},
    tcp,
    yamux::Config as YamuxConfig,
    Multiaddr, Swarm, SwarmBuilder, TransportError,
};
use log::info;
use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use crate::network::{
    config::{NetworkConfig, IDENTIFY_PROTOCOL_VERSION},
    identity::{Network, RendezvousBehaviourImpl},
    utils::get_external_addrs,
    IdentityImpl, NetworkEvent, NetworkId,
};

use super::protocol::DecentNetProtocol;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "NetworkEvent")]
pub struct DecentNetworkBehaviour {
    pub ping: Ping,
    pub floodsub: Floodsub,
    pub kademlia: Kademlia<MemoryStore>,
    pub mdns: Toggle<Mdns>,
    pub identify: Identify,
    pub protocol: RequestResponse<DecentNetProtocol>,
    pub relay: Either<RelayServer, RelayClient>,
    pub dcutr: Toggle<DCUtR>,
    pub rendezvous: Either<RendezvousClientBehaviour, RendezvousServerBehaviour>,
    // pub requests: HashSet<(OutboundRequestId, NetworkId, bool)>,
}

impl AsMut<DecentNetworkBehaviour> for DecentNetworkBehaviour {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl Network {
    fn build_behaviour(
        &self,
        config: &NetworkConfig,
        relay: Option<RelayClient>,
    ) -> DecentNetworkBehaviour {
        let mdns = if !config.server_mode && config.local_discovery {
            let mdns = Mdns::new(Default::default(), self.id());
            let mdns = if let Ok(mdns) = mdns {
                Some(mdns)
            } else {
                None
            };
            Toggle::from(mdns)
        } else {
            Toggle::from(None)
        };
        DecentNetworkBehaviour {
            ping: Ping::new(PingConfig::new()),
            floodsub: Floodsub::new(self.id()),
            kademlia: {
                let store = MemoryStore::new(self.id());
                Kademlia::new(self.id(), store)
            },
            mdns,
            identify: Identify::new(IdentifyConfig::new(
                IDENTIFY_PROTOCOL_VERSION.to_string(),
                self.public_key(),
            )),
            relay: {
                if config.server_mode {
                    //TODO: Make this configurable from config file
                    let config = RelayConfig {
                        max_reservations: 1024,
                        max_circuits: 1024,
                        max_circuits_per_peer: 8,
                        max_circuit_duration: Duration::from_secs(30 * 60),
                        max_circuit_bytes: (1 << 17) * 8 * 10,
                        ..Default::default()
                    };
                    Either::Left(RelayServer::new(self.id(), config))
                } else {
                    Either::Right(relay.unwrap())
                }
            },
            dcutr: if !config.server_mode {
                Toggle::from(Some(DCUtR::new(self.id())))
            } else {
                Toggle::from(None)
            },
            protocol: {
                let cfg = RequestResponseConfig::default()
                    .with_request_timeout(Duration::from_secs(30))
                    .to_owned();
                RequestResponse::<DecentNetProtocol>::new(DecentNetProtocol(), cfg)
            },
            rendezvous: self.rendezvous_behaviour(config.server_mode),
            // config,
            // requests: HashSet::new(),
        }
    }

    pub fn build_swarm(&self, config: &NetworkConfig) -> Swarm<DecentNetworkBehaviour> {
        let keypair = self.clone().keypair();
        let builder = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().port_reuse(true).nodelay(true),
                NoiseConfig::new,
                YamuxConfig::default,
            )
            .unwrap();
        if config.server_mode {
            builder
                .with_relay_client(NoiseConfig::new, YamuxConfig::default)
                .unwrap()
                .with_behaviour(|_, relay_behaviour| {
                    self.build_behaviour(config, Some(relay_behaviour))
                })
                .unwrap()
                .with_swarm_config(|scfg| {
                    scfg.with_dial_concurrency_factor(10_u8.try_into().unwrap())
                })
                .build()
        } else {
            builder
                .with_behaviour(|_| self.build_behaviour(config, None))
                .unwrap()
                .with_swarm_config(|scfg| {
                    scfg.with_dial_concurrency_factor(10_u8.try_into().unwrap())
                })
                .build()
        }
    }

    pub fn start_listening<TBehaviour: NetworkBehaviour>(
        swarm: &mut Swarm<TBehaviour>,
        config: &NetworkConfig,
    ) -> (
        Result<ListenerId, TransportError<io::Error>>,
        Result<ListenerId, TransportError<io::Error>>,
    ) {
        let ipv4_addr = Multiaddr::empty()
            .with(Protocol::Ip4(Ipv4Addr::UNSPECIFIED))
            .with(Protocol::Tcp(config.default_port));
        let ipv6_addr = Multiaddr::empty()
            .with(Protocol::from(Ipv6Addr::UNSPECIFIED))
            .with(Protocol::Tcp(config.default_port));
        if !config.server_mode && !config.relay_dial_mode {
            let relayed_addr = Self::get_relayed_listening_addr(config);
            info!("Listening on relay address: {}", relayed_addr);
            Swarm::listen_on(swarm, relayed_addr).expect("can listen on relayed addr");
        };
        let ipv4_listner = Swarm::listen_on(swarm, ipv4_addr);
        let ipv6_listner = Swarm::listen_on(swarm, ipv6_addr);

        (ipv4_listner, ipv6_listner)
    }

    pub fn get_known_nodes(
        &self,
        swarm: &mut Swarm<DecentNetworkBehaviour>,
        config: &NetworkConfig,
    ) -> Vec<(NetworkId, Vec<Multiaddr>)> {
        let boot_node_ids = config
            .boot_nodes
            .clone()
            .iter()
            .map(|boot_node| boot_node.network_id)
            .collect::<Vec<_>>();
        let mut unique_nodes = vec![];
        let kad = &mut swarm.behaviour_mut().kademlia;
        let nodes = kad.kbuckets();
        nodes.for_each(|bucketref| {
            bucketref.iter().for_each(|refview| {
                let network_id = refview.node.key.preimage();
                let addr = &(refview.node.value).clone().into_vec();
                let addr = get_external_addrs(addr);
                if !boot_node_ids.contains(network_id) {
                    unique_nodes.push((*network_id, addr));
                }
            });
        });
        unique_nodes
    }

    pub fn load_nodes(
        swarm: &mut Swarm<DecentNetworkBehaviour>,
        nodes: Vec<(NetworkId, Vec<Multiaddr>)>,
    ) {
        for node in nodes {
            for addr in node.1 {
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&node.0, addr.clone());
                swarm.dial(addr).unwrap();
            }
        }
    }

    pub fn get_relayed_listening_addr(config: &NetworkConfig) -> Multiaddr {
        let relay_node = config.boot_nodes.first().unwrap().clone();
        let relayed_addr = relay_node
            .multiaddr
            .with(Protocol::P2p(relay_node.network_id))
            .with(Protocol::P2pCircuit);
        relayed_addr
    }
}

#[cfg(test)]
mod tests {
    use crate::network::{config::NetworkConfig, identity::Network, BootNode};

    #[test]
    fn test_relayed_listening_addr() {
        let addr_str = "/ip4/127.0.0.1/tcp/26117";
        let relay_id = "12D3KooWPjceQrSwdWXPyLLeABRXmuqt69Rg3sBYbU1Nft9HyQ6X";
        let net_con = NetworkConfig {
            boot_nodes: vec![BootNode {
                network_id: relay_id.parse().unwrap(),
                multiaddr: addr_str.parse().unwrap(),
            }],
            ..Default::default()
        };
        let addr = Network::get_relayed_listening_addr(&net_con).to_string();
        assert_eq!(addr, format!("{}/p2p/{}/p2p-circuit", addr_str, relay_id));
    }
}
