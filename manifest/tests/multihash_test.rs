use cid::Cid;
use multihash::{Code, MultihashDigest};

#[test]
pub fn test_multihash() {
    let data = b"Hello, world!";

    let digest = Code::Sha2_256.digest(data);
    let cid = Cid::new_v1(0x55, digest);

    // Generate a multihash from the digest
    let multihash = cid.to_bytes();

    let cid2 = Cid::try_from(multihash).unwrap();
    cid2.to_string();

    // Print the multihash as hex string
    let multihash_hex = cid
        .to_string_of_base(cid::multibase::Base::Base58Btc)
        .unwrap();
    let bytes = cid2.hash().to_bytes();
    assert_eq!(digest.to_bytes(), bytes);
    println!("Multihash: {}", multihash_hex);
}
