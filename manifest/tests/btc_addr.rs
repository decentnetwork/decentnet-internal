use bitcoin::{
    hashes::sha256,
    key::Keypair,
    secp256k1::{rand, Message, Secp256k1, XOnlyPublicKey},
    Address,
};

#[test]
pub fn test_site_addr() {
    let secp = Secp256k1::new();
    // let (_, pks) = secp.generate_keypair(&mut rand::thread_rng());
    // let pk = PublicKey::new(pks);
    let key_pair = Keypair::new(&secp, &mut rand::thread_rng());
    let xonly = XOnlyPublicKey::from_keypair(&key_pair);
    let addr = Address::p2tr(&secp, xonly.0, None, bitcoin::Network::Bitcoin);
    println!("addr: {}", addr);

    let msg = b"Hello World!";
    let msg = Message::from_hashed_data::<sha256::Hash>(msg);
    let sig = secp.sign_schnorr_with_rng(&msg, &key_pair, &mut rand::thread_rng());
    println!("sig: {:?}", sig);

    println!("xonly: {:?}", xonly.0);

    // let pk = PublicKey::from(&secp, &xonly).unwrap();

    let verify = secp.verify_schnorr(&sig, &msg, &xonly.0);
    println!("verify: {:?}", verify);
}
