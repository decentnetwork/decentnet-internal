use cid::{
    multihash::{Code, MultihashDigest},
    Cid,
};
use multihash::Multihash;

#[test]
pub fn test_multihash() {
    let data = b"Hello 1NGmnrtExYz4TApULNuiv4VpWS3rjKvnYX!";

    let digest = Code::Blake3_256.digest(data);
    let cid = Cid::new_v1(0x55, digest);

    // Generate a multihash from the digest
    let multihash = cid.to_bytes();

    let hash = Multihash::<64>::from_bytes(&cid.hash().to_bytes()).unwrap();
    println!("Multihash: {:x?}\n", &hash.to_bytes());

    let cid2 = Cid::try_from(multihash).unwrap();
    assert_eq!(cid, cid2);
    println!("Cid: {:?}", cid2.version());
    let cid_str = cid2.to_string();
    println!("Cid: {}", cid_str);

    // Print the multihash as hex string
    let multihash_hex = cid
        .to_string_of_base(cid::multibase::Base::Base58Btc)
        .unwrap();
    let bytes = cid2.hash().to_bytes();
    assert_eq!(digest.to_bytes(), bytes);
    println!("Multihash: {}", multihash_hex);
}
