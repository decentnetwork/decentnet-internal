use std::{
    net::{IpAddr, Ipv6Addr},
    str::FromStr,
};

use libp2p::{identity::ed25519::SecretKey, Multiaddr, PeerId, Swarm};

use crate::network::{
    behaviour::DecentNetworkBehaviour, config::NetworkConfig, identity::Network, BootNode,
    IdentityImpl, NetworkId,
};

const IP_V6_FOR_TEST: &str = "2607:f8b0:4006:81e::200e";

pub fn initialise_default() -> Result<Swarm<DecentNetworkBehaviour>, &'static str> {
    let private_key = SecretKey::generate();
    let net_cfg = NetworkConfig {
        boot_nodes: load_bootnodes("boot_nodes.txt"),
        ..Default::default()
    };
    initialise_with_private_key(net_cfg, private_key)
}

pub fn initialise(config: NetworkConfig) -> Result<Swarm<DecentNetworkBehaviour>, &'static str> {
    let private_key = SecretKey::generate();
    initialise_with_private_key(config, private_key)
}

pub fn initialise_with_network(
    network_config: NetworkConfig,
    network: Network,
) -> Result<Swarm<DecentNetworkBehaviour>, &'static str> {
    let swarm = network.build_swarm(&network_config);
    Ok(swarm)
}

pub fn initialise_with_private_key(
    network_config: NetworkConfig,
    private_key: SecretKey,
) -> Result<Swarm<DecentNetworkBehaviour>, &'static str> {
    let network = {
        let bytes: Vec<u8> = private_key.as_ref().to_vec();
        Network::from_bytes(bytes)
    };
    initialise_with_network(network_config, network)
}

pub fn list_nodes(
    swarm: &mut Swarm<DecentNetworkBehaviour>,
    config: &NetworkConfig,
) -> Vec<String> {
    DecentNetworkBehaviour::list_nodes(swarm, config)
        .into_iter()
        .map(|node| {
            let peer_id = node.network_id;
            let address = node.multiaddr.iter().fold(String::new(), |mut a, b| {
                a.push_str("::");
                a.push_str(b);
                a
            });
            format!("{} {}", peer_id, address)
        })
        .collect()
}

pub fn get_nodes_from_bootnodes(swarm: &mut Swarm<DecentNetworkBehaviour>, config: &NetworkConfig) {
    DecentNetworkBehaviour::get_nodes_from_bootnodes(swarm, config)
}

pub fn get_nodes(swarm: &mut Swarm<DecentNetworkBehaviour>, network_id: &str) {
    DecentNetworkBehaviour::get_nodes(swarm, &PeerId::from_str(network_id).unwrap())
}

pub fn dial_addr(swarm: &mut Swarm<DecentNetworkBehaviour>, addr: &str) -> Result<(), String> {
    DecentNetworkBehaviour::dial_addr(swarm, addr)
}

pub fn ping(network: &mut DecentNetworkBehaviour, network_id: &str) {
    DecentNetworkBehaviour::ping(network, &PeerId::from_str(network_id).unwrap())
}

pub fn external_addresses(swarm: &mut Swarm<DecentNetworkBehaviour>) -> Vec<String> {
    DecentNetworkBehaviour::external_addresses(swarm)
        .map(|addr| addr.to_string())
        .collect()
}

pub fn bootnodes(_: &mut Swarm<DecentNetworkBehaviour>, config: &NetworkConfig) -> Vec<String> {
    config
        .boot_nodes
        .clone()
        .into_iter()
        .map(|boot_node| {
            let peer_id = boot_node.network_id;
            let address = boot_node.multiaddr.to_string();
            format!("{}::{}", peer_id, address)
        })
        .collect()
}

pub fn listeners(swarm: &mut Swarm<DecentNetworkBehaviour>) -> Vec<String> {
    swarm
        .listeners()
        .map(|listener| listener.to_string())
        .collect()
}

/// format for file content
/// NETWORK_ID MULTIADDR<::>MULTIADDR
/// <::> is the separator between multiaddrs
/// First multiaddr is for ipv4 and second multiaddr is for ipv6
/// You can omit the second multiaddr if you don't have it
pub fn load_bootnodes(file_path: &str) -> Vec<BootNode> {
    let file = std::fs::read_to_string(file_path);
    if let Ok(file) = file {
        file.lines()
            .filter_map(|line| {
                let mut parts = line.split(' ');
                let network_id = NetworkId::from_str(parts.next().unwrap()).unwrap();
                let address = parts
                    .next()
                    .unwrap()
                    .split("<::>")
                    .filter(|x| !x.is_empty())
                    .collect::<Vec<_>>();
                if address.is_empty() {
                    return None;
                }
                let multiaddr = if ipv6_supported() {
                    let ip_v6 = address.iter().filter(|x| !x.starts_with("/ip6")).collect::<Vec<_>>();
                    let ip_v6 = ip_v6.get(1).get_or_insert(&&address[0]).to_string();
                    ip_v6.parse().unwrap()
                } else {
                    let ip_v4 = address.get(0).unwrap().to_string();
                    ip_v4.parse().unwrap()
                };
                Some(BootNode {
                    network_id,
                    multiaddr,
                })
            })
            .collect()
    } else {
        vec![]
    }
}

pub fn ipv6_supported() -> bool {
    let addr = IpAddr::from(Ipv6Addr::from_str(IP_V6_FOR_TEST).unwrap());
    ping::ping(addr, None, None, None, None, None).is_ok()
}
