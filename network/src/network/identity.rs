use either::Either;
use libp2p::{
    identity::{self, ed25519},
    rendezvous::client::Behaviour as RendezvousClientBehaviour,
    rendezvous::server::Behaviour as RendezvousServerBehaviour,
    rendezvous::server::Config as RendezvousServerConfig,
};

use super::{config::NetworkConfig, IdentityImpl, NetworkId};

// Used to provide Unique Node Id across Network.
// Services can use this to identify users or
// they can use their own store to provide unique names across their service.
#[derive(Default, Clone)]
pub struct Network {
    keypair: Keypair,
    pub config: NetworkConfig,
}

#[derive(Clone)]
pub struct Keypair(identity::Keypair);

impl Default for Keypair {
    fn default() -> Self {
        let private_key = ed25519::SecretKey::generate();
        let keypair = ed25519::Keypair::from(private_key);
        let keypair = identity::Keypair::from(keypair);
        Self(keypair)
    }
}

impl Network {
    pub fn keypair(self) -> identity::Keypair {
        self.keypair.0
    }
}

impl<'a> IdentityImpl<'a> for Network {
    type PublicKey = identity::PublicKey;
    fn id(&self) -> NetworkId {
        NetworkId::from_public_key(&self.keypair.0.public())
    }

    fn public_key(&self) -> Self::PublicKey {
        self.keypair.0.public()
    }

    fn gen_random_id() -> Self {
        let keypair = Keypair::default();
        Self {
            keypair,
            ..Default::default()
        }
    }

    fn gen_random_id_with_private() -> (ed25519::SecretKey, Self) {
        let private_key = ed25519::SecretKey::generate();
        let keypair = ed25519::Keypair::from(private_key.clone());
        // let keypair = identity::Keypair::Ed25519(keypair);
        let keypair = identity::Keypair::from(keypair);
        let keypair = Keypair(keypair);
        (
            private_key,
            Self {
                keypair,
                ..Default::default()
            },
        )
    }

    fn from_bytes(bytes: impl AsMut<[u8]>) -> Self {
        let private_key = ed25519::SecretKey::try_from_bytes(bytes).unwrap();
        let keypair = ed25519::Keypair::from(private_key);
        // let keypair = identity::Keypair::Ed25519(keypair);
        let keypair = identity::Keypair::from(keypair);
        let keypair = Keypair(keypair);
        Self {
            keypair,
            ..Default::default()
        }
    }

    // fn auth_key_pair(&self) -> Result<AuthenticKeypair<X25519Spec>, NoiseError> {
    //     Keypair::<X25519Spec>::new().into_authentic(&self.keypair)
    // }
}

pub trait RendezvousBehaviourImpl {
    type Behaviour;
    fn rendezvous_behaviour(&self, is_server: bool) -> Self::Behaviour;
}

impl RendezvousBehaviourImpl for Network {
    type Behaviour = Either<RendezvousClientBehaviour, RendezvousServerBehaviour>;
    fn rendezvous_behaviour(&self, is_server: bool) -> Self::Behaviour {
        if is_server {
            Either::Right(RendezvousServerBehaviour::new(
                RendezvousServerConfig::default(),
            ))
        } else {
            Either::Left(RendezvousClientBehaviour::new(self.keypair.0.clone()))
        }
    }
}

//This is the struct that will be used to represent a software client in the network
pub struct Peer {
    // keypair: identity::Keypair,
}

// Used to get Network Strength, Metrics, Update System, Relay(Only) Mode.
// impl Network {
//     pub fn id(&self) -> String {
//         NetworkId::from_public_key(self.public()).to_base58()
//     }

//     pub fn public(&self) -> identity::PublicKey {
//         self.keypair.public()
//     }

//     fn gen_random_id() -> Self {
//         let private_key = ed25519::SecretKey::generate();
//         let keypair = ed25519::Keypair::from(private_key.clone());
//         let keypair = identity::Keypair::Ed25519(keypair.clone());
//         Network { keypair }
//     }

//     fn from_bytes(bytes: impl AsMut<[u8]>) -> Self {
//         let private_key = ed25519::SecretKey::from_bytes(bytes).unwrap();
//         let keypair = ed25519::Keypair::from(private_key);
//         let keypair = identity::Keypair::Ed25519(keypair);
//         Network { keypair }
//     }
// }

// This is representation for websites and apps, typical ZeroNet Style.
// Implement a Initial App Update Service.
// struct Service {
//     keypair: identity::Keypair,
// }

// impl Identity for Service {
//     fn id(&self) -> String {
//         NetworkId::from_public_key(self.keypair.public()).to_base58()
//     }

//     fn gen_random() -> Self {
//         let private_key = ed25519::SecretKey::generate();
//         let keypair = ed25519::Keypair::from(private_key.clone());
//         let keypair = identity::Keypair::Ed25519(keypair.clone());
//         Service { keypair }
//     }

//     fn from_bytes(bytes: impl AsMut<[u8]>) -> Self {
//         let private_key = ed25519::SecretKey::from_bytes(bytes).unwrap();
//         let keypair = ed25519::Keypair::from(private_key);
//         let keypair = identity::Keypair::Ed25519(keypair);
//         Service { keypair }
//     }
// }
