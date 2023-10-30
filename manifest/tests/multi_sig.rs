use frost::{
    keys::{
        generate_with_dealer,
        repairable::{repair_share_step_1, repair_share_step_2, repair_share_step_3},
        IdentifierList,
    },
    Error, Identifier, Secp256K1Sha256,
};
use frost_secp256k1 as frost;
use rand::thread_rng;
use std::collections::{BTreeMap, HashMap};

#[test]
fn test_multisig() {
    let mut rng = thread_rng();
    let max_signers = 2;
    let min_signers = 2;
    let (shares, pubkey_package) = generate_with_dealer(
        max_signers,
        min_signers,
        frost::keys::IdentifierList::Default,
        &mut rng,
    )
    .unwrap();

    // Verifies the secret shares from the dealer and store them in a HashMap.
    // In practice, the KeyPackages must be sent to its respective participants
    // through a confidential and authenticated channel.
    let mut key_packages: HashMap<_, _> = HashMap::new();

    for (identifier, secret_share) in shares {
        let key_package = frost::keys::KeyPackage::try_from(secret_share).unwrap();
        key_packages.insert(identifier, key_package);
    }

    let mut nonces_map = HashMap::new();
    let mut commitments_map = BTreeMap::new();

    ////////////////////////////////////////////////////////////////////////////
    // Round 1: generating nonces and signing commitments for each participant
    ////////////////////////////////////////////////////////////////////////////

    // In practice, each iteration of this loop will be executed by its respective participant.
    for participant_index in 1..(min_signers + 1) {
        let participant_identifier = participant_index.try_into().expect("should be nonzero");
        let _key_package = &key_packages[&participant_identifier];
        // Generate one (1) nonce and one SigningCommitments instance for each
        // participant, up to _threshold_.
        let (nonces, commitments) = frost::round1::commit(
            key_packages[&participant_identifier].secret_share(),
            &mut rng,
        );
        // In practice, the nonces must be kept by the participant to use in the
        // next round, while the commitment must be sent to the coordinator
        // (or to every other participant if there is no coordinator) using
        // an authenticated channel.
        nonces_map.insert(participant_identifier, nonces);
        commitments_map.insert(participant_identifier, commitments);
    }

    // This is what the signature aggregator / coordinator needs to do:
    // - decide what message to sign
    // - take one (unused) commitment per signing participant
    let mut signature_shares = HashMap::new();
    let message = "message to sign".as_bytes();
    let signing_package = frost::SigningPackage::new(commitments_map, message);

    ////////////////////////////////////////////////////////////////////////////
    // Round 2: each participant generates their signature share
    ////////////////////////////////////////////////////////////////////////////

    // In practice, each iteration of this loop will be executed by its respective participant.
    for participant_identifier in nonces_map.keys() {
        let key_package = &key_packages[participant_identifier];

        let nonces = &nonces_map[participant_identifier];

        // Each participant generates their signature share.
        let signature_share = frost::round2::sign(&signing_package, nonces, key_package).unwrap();

        // In practice, the signature share must be sent to the Coordinator
        // using an authenticated channel.
        signature_shares.insert(*participant_identifier, signature_share);
    }

    ////////////////////////////////////////////////////////////////////////////
    // Aggregation: collects the signing shares from all participants,
    // generates the final signature.
    ////////////////////////////////////////////////////////////////////////////

    // Aggregate (also verifies the signature shares)
    let group_signature =
        frost::aggregate(&signing_package, &signature_shares, &pubkey_package).unwrap();

    println!("group signature: {:?}", group_signature);

    // Check that the threshold signature can be verified by the group public
    // key (the verification key).
    let is_signature_valid = pubkey_package
        .group_public()
        .verify(message, &group_signature)
        .is_ok();
    assert!(is_signature_valid);
}

#[test]
fn recover_secret() {
    let rng = thread_rng();
    let max_signers = 3;
    let min_signers = 2;
    let (shares, _pubkeys): (_, _) =
        generate_with_dealer(max_signers, min_signers, IdentifierList::Default, rng).unwrap();

    let participant = &shares[&Identifier::try_from(1).unwrap()];

    let helper = &shares[&Identifier::try_from(2).unwrap()];
    let helper_1 = &shares[&Identifier::try_from(3).unwrap()];

    let helpers = [*helper.identifier(), *helper_1.identifier()];

    let mut rng = thread_rng();
    let helper_delta = repair_share_step_1::<Secp256K1Sha256, _>(
        &helpers,
        helper,
        &mut rng,
        *participant.identifier(),
    )
    .unwrap();

    let helper_1_delta = repair_share_step_1::<Secp256K1Sha256, _>(
        &helpers,
        helper_1,
        &mut rng,
        *participant.identifier(),
    )
    .unwrap();
    let helper_sigma = repair_share_step_2(&[helper_delta[&helpers[0]]]);

    let helper_1_sigma = repair_share_step_2(&[helper_1_delta[&helpers[1]]]);

    let secret_share = repair_share_step_3(
        &[helper_sigma, helper_1_sigma],
        *participant.identifier(),
        participant.commitment(),
    );

    assert_ne!(secret_share.secret(), participant.secret());
}

#[test]
fn recover_secret_1() {
    let mut rng = thread_rng();
    let max_signers = 2;
    let min_signers = 2;
    let (shares, _pubkeys): (_, _) =
        generate_with_dealer(max_signers, min_signers, IdentifierList::Default, &mut rng).unwrap();

    let participant = &shares[&Identifier::try_from(1).unwrap()];

    let helper = &shares[&Identifier::try_from(1).unwrap()];
    let helper_1 = &shares[&Identifier::try_from(2).unwrap()];

    let helpers = [*helper.identifier(), *helper_1.identifier()];

    let helper_delta = repair_share_step_1::<Secp256K1Sha256, _>(
        &helpers,
        helper,
        &mut rng,
        *participant.identifier(),
    )
    .unwrap();

    let helper_1_delta = repair_share_step_1::<Secp256K1Sha256, _>(
        &helpers,
        helper_1,
        &mut rng,
        *participant.identifier(),
    )
    .unwrap();
    let helper_sigma =
        repair_share_step_2(&[helper_delta[&helpers[0]], helper_1_delta[&helpers[0]]]);

    let helper_1_sigma =
        repair_share_step_2(&[helper_1_delta[&helpers[1]], helper_delta[&helpers[1]]]);

    let secret_share = repair_share_step_3(
        &[helper_sigma, helper_1_sigma],
        *participant.identifier(),
        participant.commitment(),
    );

    assert_eq!(secret_share.secret(), participant.secret());
}

#[test]
fn recover_secret_2() {
    let mut rng = thread_rng();
    let max_signers = 3;
    let min_signers = 2; // This is to make sure this test fails at the right point
    let (shares, _pubkeys): (HashMap<_, _>, _) = frost::keys::generate_with_dealer(
        max_signers,
        min_signers,
        frost::keys::IdentifierList::Default,
        &mut rng,
    )
    .unwrap();

    let helper = Identifier::try_from(3).unwrap();

    let out = repair_share_step_1::<Secp256K1Sha256, _>(
        &[helper],
        &shares[&helper],
        &mut rng,
        Identifier::try_from(2).unwrap(),
    );

    assert!(out.is_err());
    assert!(out == Err(Error::InvalidMinSigners))
}

#[test]
fn recover_secret_3() {
    let mut rng = thread_rng();

    let max_signers = 3;
    let min_signers = 2;
    let (shares, _pubkeys): (_, _) = frost::keys::generate_with_dealer(
        max_signers,
        min_signers,
        frost::keys::IdentifierList::Default,
        &mut rng,
    )
    .unwrap();

    // Try to recover a share

    // Signer 2 will lose their share
    // Signer 1, 4 and 5 will help signer 2 to recover their share

    let helper_1 = &shares[&Identifier::try_from(1).unwrap()];
    let helper_4 = &shares[&Identifier::try_from(3).unwrap()];
    let participant = &shares[&Identifier::try_from(2).unwrap()];

    let helpers: [_; 2] = [*helper_1.identifier(), *helper_4.identifier()];

    // Each helper generates random values for each helper

    let helper_1_deltas = repair_share_step_1::<Secp256K1Sha256, _>(
        &helpers,
        helper_1,
        &mut rng,
        *participant.identifier(),
    )
    .unwrap();
    let helper_4_deltas = repair_share_step_1::<Secp256K1Sha256, _>(
        &helpers,
        helper_4,
        &mut rng,
        *participant.identifier(),
    )
    .unwrap();

    // Each helper calculates their sigma from the random values received from the other helpers

    let helper_1_sigma =
        repair_share_step_2(&[helper_1_deltas[&helpers[0]], helper_4_deltas[&helpers[0]]]);
    let helper_4_sigma =
        repair_share_step_2(&[helper_1_deltas[&helpers[1]], helper_4_deltas[&helpers[1]]]);

    // The participant wishing to recover their share sums the sigmas sent from all helpers

    let participant_recovered_share = repair_share_step_3(
        &[helper_1_sigma, helper_4_sigma],
        *participant.identifier(),
        participant.commitment(),
    );

    // TODO: assert on commitment equality as well once updates have been made to VerifiableSecretSharingCommitment
    assert!(participant.secret() == participant_recovered_share.secret())
}

#[test]
fn recover_secret_4() {
    let mut rng = thread_rng();

    let max_signers = 5;
    let min_signers = 3;
    let (shares, _pubkeys): (_, _) = frost::keys::generate_with_dealer(
        max_signers,
        min_signers,
        frost::keys::IdentifierList::Default,
        &mut rng,
    )
    .unwrap();

    // Try to recover a share

    // Signer 2 will lose their share
    // Signer 1, 4 and 5 will help signer 2 to recover their share

    let helper_1 = &shares[&Identifier::try_from(1).unwrap()];
    let helper_4 = &shares[&Identifier::try_from(4).unwrap()];
    let helper_5 = &shares[&Identifier::try_from(5).unwrap()];
    let participant = &shares[&Identifier::try_from(2).unwrap()];

    let helpers: [_; 3] = [
        *helper_1.identifier(),
        *helper_4.identifier(),
        *helper_5.identifier(),
    ];

    // Each helper generates random values for each helper

    let helper_1_deltas = repair_share_step_1::<Secp256K1Sha256, _>(
        &helpers,
        helper_1,
        &mut rng,
        *participant.identifier(),
    )
    .unwrap();
    let helper_4_deltas = repair_share_step_1::<Secp256K1Sha256, _>(
        &helpers,
        helper_4,
        &mut rng,
        *participant.identifier(),
    )
    .unwrap();
    let helper_5_deltas = repair_share_step_1::<Secp256K1Sha256, _>(
        &helpers,
        helper_5,
        &mut rng,
        *participant.identifier(),
    )
    .unwrap();

    // Each helper calculates their sigma from the random values received from the other helpers

    let helper_1_sigma = repair_share_step_2(&[
        helper_1_deltas[&helpers[0]],
        helper_4_deltas[&helpers[0]],
        helper_5_deltas[&helpers[0]],
    ]);
    let helper_4_sigma = repair_share_step_2(&[
        helper_1_deltas[&helpers[1]],
        helper_4_deltas[&helpers[1]],
        helper_5_deltas[&helpers[1]],
    ]);
    let helper_5_sigma = repair_share_step_2(&[
        helper_1_deltas[&helpers[2]],
        helper_4_deltas[&helpers[2]],
        helper_5_deltas[&helpers[2]],
    ]);

    // The participant wishing to recover their share sums the sigmas sent from all helpers

    let participant_recovered_share = repair_share_step_3(
        &[helper_1_sigma, helper_4_sigma, helper_5_sigma],
        *participant.identifier(),
        participant.commitment(),
    );

    // TODO: assert on commitment equality as well once updates have been made to VerifiableSecretSharingCommitment
    assert!(participant.secret() == participant_recovered_share.secret())
}
