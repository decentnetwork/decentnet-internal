use frost::{
    keys::{
        generate_with_dealer,
        repairable::{repair_share_step_1, repair_share_step_2, repair_share_step_3},
        resharing::{reshare_step_1, reshare_step_2, SecretSubshare},
        IdentifierList, PublicKeyPackage, SecretShare,
    },
    Error, Identifier, Ristretto255Sha512 as Cipher,
};
use frost_ristretto255 as frost;
use rand::thread_rng;
use std::collections::BTreeMap;

fn gen_keys(
    min_signers: u16,
    max_signers: u16,
    mut rng: impl rand::RngCore + rand::CryptoRng,
) -> (BTreeMap<Identifier, SecretShare>, PublicKeyPackage) {
    generate_with_dealer(max_signers, min_signers, IdentifierList::Default, &mut rng).unwrap()
}

/// Generates a group signature with 2 signers, 2 of which are required to sign.
/// The signature is then verified by the group public key.
#[test]
fn test_multisig() {
    let mut rng = thread_rng();
    let max_signers = 2;
    let min_signers = 2;
    let (shares, pubkey_package) = gen_keys(max_signers, min_signers, &mut rng);

    // Verifies the secret shares from the dealer and store them in a HashMap.
    // In practice, the KeyPackages must be sent to its respective participants
    // through a confidential and authenticated channel.
    let mut key_packages = BTreeMap::new();

    for (identifier, secret_share) in shares {
        let key_package = frost::keys::KeyPackage::try_from(secret_share).unwrap();
        key_packages.insert(identifier, key_package);
    }

    let mut nonces_map = BTreeMap::new();
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
            key_packages[&participant_identifier].signing_share(),
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
    let mut signature_shares = BTreeMap::new();
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
        .verifying_key()
        .verify(message, &group_signature)
        .is_ok();
    assert!(is_signature_valid);
}

/// Share recovery test with 3 signers, 2 of which are required to recover the share.
/// Signer 1 will lose their share.
/// Signer 2 and 3 will help signer 1 to recover their share.
#[test]
fn recover_secret() {
    let rng = thread_rng();
    let max_signers = 3;
    let min_signers = 2;
    let (shares, _pubkeys) = gen_keys(min_signers, max_signers, rng);

    let participant = &shares[&Identifier::try_from(1).unwrap()];

    let helper = &shares[&Identifier::try_from(2).unwrap()];
    let helper_1 = &shares[&Identifier::try_from(3).unwrap()];

    let helpers = [*helper.identifier(), *helper_1.identifier()];

    let mut rng = thread_rng();
    let helper_delta =
        repair_share_step_1::<Cipher, _>(&helpers, helper, &mut rng, *participant.identifier())
            .unwrap();

    let helper_1_delta =
        repair_share_step_1::<Cipher, _>(&helpers, helper_1, &mut rng, *participant.identifier())
            .unwrap();
    let helper_sigma = repair_share_step_2(&[helper_delta[&helpers[0]]]);

    let helper_1_sigma = repair_share_step_2(&[helper_1_delta[&helpers[1]]]);

    let secret_share = repair_share_step_3(
        &[helper_sigma, helper_1_sigma],
        *participant.identifier(),
        participant.commitment(),
    );

    assert_ne!(secret_share.signing_share(), participant.signing_share());
}

/// Share recovery test with 2 signers, 2 of which are required to recover the share.
/// Signer 2 will lose their share.
/// Signer 1 will help signer 2 to recover their share.
/// Share recovery should fail because the minimum number of signers required to recover the share is 2.
#[test]
fn recover_secret_1() {
    let mut rng = thread_rng();
    let max_signers = 2;
    let min_signers = 2;
    let (shares, _pubkeys) = gen_keys(max_signers, min_signers, &mut rng);

    let participant = &shares[&Identifier::try_from(1).unwrap()];

    let helper = &shares[&Identifier::try_from(1).unwrap()];

    let helpers = [*helper.identifier()];

    let helper_delta =
        repair_share_step_1::<Cipher, _>(&helpers, helper, &mut rng, *participant.identifier());

    assert!(helper_delta.is_err());
    assert!(helper_delta == Err(Error::InvalidMinSigners))
}

/// Share recovery test with 3 signers, 2 of which are required to recover the share.
/// Signer 2 will lose their share.
/// Signer 1 will help signer 2 to recover their share.
/// Share recovery should fail because the minimum number of signers required to recover the share is 2.
#[test]
fn recover_secret_2() {
    let mut rng = thread_rng();
    let max_signers = 3;
    let min_signers = 2; // This is to make sure this test fails at the right point
    let (shares, _pubkeys): (BTreeMap<_, _>, _) = gen_keys(min_signers, max_signers, &mut rng);

    let helper = Identifier::try_from(3).unwrap();

    let out = repair_share_step_1::<Cipher, _>(
        &[helper],
        &shares[&helper],
        &mut rng,
        Identifier::try_from(2).unwrap(),
    );

    assert!(out.is_err());
    assert!(out == Err(Error::InvalidMinSigners))
}

/// Share recovery test with 5 signers, 2 of which are required to recover the share.
/// Signer 2 will lose their share.
/// Signer 1 and 4 will help signer 2 to recover their share.
/// Signer 3 and 5 will not participate in the recovery.
#[test]
fn recover_secret_3() {
    let mut rng = thread_rng();

    let max_signers = 3;
    let min_signers = 2;
    let (shares, _pubkeys) = gen_keys(min_signers, max_signers, &mut rng);

    // Try to recover a share

    // Signer 2 will lose their share
    // Signer 1, 4 and 5 will help signer 2 to recover their share

    let helper_1 = &shares[&Identifier::try_from(1).unwrap()];
    let helper_4 = &shares[&Identifier::try_from(3).unwrap()];
    let participant = &shares[&Identifier::try_from(2).unwrap()];

    let helpers: [_; 2] = [*helper_1.identifier(), *helper_4.identifier()];

    // Each helper generates random values for each helper

    let helper_1_deltas =
        repair_share_step_1::<Cipher, _>(&helpers, helper_1, &mut rng, *participant.identifier())
            .unwrap();
    let helper_4_deltas =
        repair_share_step_1::<Cipher, _>(&helpers, helper_4, &mut rng, *participant.identifier())
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
    assert!(participant.signing_share() == participant_recovered_share.signing_share())
}

/// Share recovery test with 5 signers, 3 of which are required to recover the share.
/// Signer 2 will lose their share.
/// Signer 1, 4 and 5 will help signer 2 to recover their share.
#[test]
fn recover_secret_4() {
    let mut rng = thread_rng();

    let max_signers = 5;
    let min_signers = 3;
    let (shares, _pubkeys) = gen_keys(min_signers, max_signers, &mut rng);

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

    let helper_1_deltas =
        repair_share_step_1::<Cipher, _>(&helpers, helper_1, &mut rng, *participant.identifier())
            .unwrap();
    let helper_4_deltas =
        repair_share_step_1::<Cipher, _>(&helpers, helper_4, &mut rng, *participant.identifier())
            .unwrap();
    let helper_5_deltas =
        repair_share_step_1::<Cipher, _>(&helpers, helper_5, &mut rng, *participant.identifier())
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
    assert!(participant.signing_share() == participant_recovered_share.signing_share())
}

/// Initial Min Signers: 3, Max Signers: 5
/// New     Min Signers: 2, Max Signers: 5
/// Signers 1, 2, 4 will participate in resharing.
/// They will reshare the key amongst themselves, plus new signer 5.
/// Signer 3 will be excluded.
/// The threshold will be changed from 3 to 2.
/// Each helper generates their random coefficients and commitments.
/// All signers should compute the same group pubkeys.
/// The new pubkey package should be the same group key as the old one,
/// but with new coefficients and shares.
#[test]
fn reshare_verify_key_5() {
    let mut rng = thread_rng();

    let max_signers = 5;
    let old_min_signers = 3;
    let (old_shares, old_pubkeys) = gen_keys(old_min_signers, max_signers, &mut rng);

    // Signer 1, 2, and 4 will participate in resharing.
    let helper_1 = &old_shares[&Identifier::try_from(1).unwrap()];
    let helper_2 = &old_shares[&Identifier::try_from(2).unwrap()];
    let helper_4 = &old_shares[&Identifier::try_from(4).unwrap()];

    // They will reshare the key amongst themselves, plus new signer 5.
    // Signer 3 will be excluded.
    let new_signer_5_ident = Identifier::try_from(5).unwrap();
    let new_signer_idents = [
        *helper_1.identifier(),
        *helper_2.identifier(),
        *helper_4.identifier(),
        new_signer_5_ident,
    ];

    // The threshold will be changed from 3 to 2.
    let new_min_signers = 2;

    // Each helper generates their random coefficients and commitments.
    let helper_1_subshares = reshare_step_1(
        helper_1.signing_share(),
        &mut rng,
        new_min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 1");

    let helper_2_subshares = reshare_step_1(
        helper_2.signing_share(),
        &mut rng,
        new_min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 2");

    let helper_4_subshares = reshare_step_1(
        helper_4.signing_share(),
        &mut rng,
        new_min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 4");

    let all_subshares = BTreeMap::from([
        (helper_1.identifier(), helper_1_subshares),
        (helper_2.identifier(), helper_2_subshares),
        (helper_4.identifier(), helper_4_subshares),
    ]);

    // Sort the subshares into a map of `recipient => sender => subshare`.
    let received_subshares = new_signer_idents
        .into_iter()
        .map(|recipient_id| {
            let received_subshares = all_subshares
                .iter()
                .map(|(&sender_id, sender_shares)| {
                    (*sender_id, sender_shares[&recipient_id].clone())
                })
                .collect::<BTreeMap<_, _>>();
            (recipient_id, received_subshares)
        })
        .collect::<BTreeMap<_, BTreeMap<_, SecretSubshare>>>();

    // Recipients of the resharing can now validate and compute their new shares.

    let (new_seckeys_1, new_pubkeys_1) = reshare_step_2(
        *helper_1.identifier(),
        &old_pubkeys,
        new_min_signers,
        new_signer_idents.as_slice(),
        &received_subshares[&helper_1.identifier()],
    )
    .expect("error computing reshared share for signer 1");

    let (new_seckeys_2, new_pubkeys_2) = reshare_step_2(
        *helper_2.identifier(),
        &old_pubkeys,
        new_min_signers,
        &new_signer_idents,
        &received_subshares[&helper_2.identifier()],
    )
    .expect("error computing reshared share for signer 2");

    let (new_seckeys_4, new_pubkeys_4) = reshare_step_2(
        *helper_4.identifier(),
        &old_pubkeys,
        new_min_signers,
        &new_signer_idents,
        &received_subshares[&helper_4.identifier()],
    )
    .expect("error computing reshared share for signer 4");

    let (new_seckeys_5, new_pubkeys_5) = reshare_step_2(
        new_signer_5_ident,
        &old_pubkeys,
        new_min_signers,
        &new_signer_idents,
        &received_subshares[&new_signer_5_ident],
    )
    .expect("error computing reshared share for signer 5");

    // all signers should compute the same group pubkeys.
    assert_eq!(new_pubkeys_1, new_pubkeys_2);
    assert_eq!(new_pubkeys_1, new_pubkeys_4);
    assert_eq!(new_pubkeys_1, new_pubkeys_5);
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_2.verifying_key());
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_4.verifying_key());
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_5.verifying_key());

    // The new pubkey package should be the same group key as the old one,
    // but with new coefficients and shares.
    assert_eq!(new_pubkeys_1.verifying_key(), old_pubkeys.verifying_key());
    assert_ne!(
        new_pubkeys_1.verifying_shares(),
        old_pubkeys.verifying_shares()
    );

    assert_eq!(new_seckeys_1.min_signers(), &new_min_signers);
}

/// Initial Min Signers: 3, Max Signers: 5
/// New     Min Signers: 3, Max Signers: 5
/// Signers 1, 2, 4 will participate in resharing.
/// They will reshare the key amongst themselves.
/// Signer 3 will be excluded.
/// The min signers threshold won't be changed.
/// Each helper generates their random coefficients and commitments.
/// All signers should compute the same group pubkeys.
/// The new pubkey package should be the same group key as the old one,
/// but with new coefficients and shares.
#[test]
fn reshare_verify_key_6() {
    let mut rng = thread_rng();

    let max_signers = 5;
    let min_signers = 3;
    let (old_shares, old_pubkeys) = gen_keys(min_signers, max_signers, &mut rng);

    // Signer 1, 2, and 4 will participate in resharing.
    let helper_1 = &old_shares[&Identifier::try_from(1).unwrap()];
    let helper_2 = &old_shares[&Identifier::try_from(2).unwrap()];
    let helper_4 = &old_shares[&Identifier::try_from(4).unwrap()];

    // They will reshare the key amongst themselves.
    // Signer 3 will be excluded.
    let new_signer_idents = [
        *helper_1.identifier(),
        *helper_2.identifier(),
        *helper_4.identifier(),
    ];

    // Each helper generates their random coefficients and commitments.
    let helper_1_subshares = reshare_step_1(
        helper_1.signing_share(),
        &mut rng,
        min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 1");

    let helper_2_subshares = reshare_step_1(
        helper_2.signing_share(),
        &mut rng,
        min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 2");

    let helper_4_subshares = reshare_step_1(
        helper_4.signing_share(),
        &mut rng,
        min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 4");

    let all_subshares = BTreeMap::from([
        (helper_1.identifier(), helper_1_subshares),
        (helper_2.identifier(), helper_2_subshares),
        (helper_4.identifier(), helper_4_subshares),
    ]);

    // Sort the subshares into a map of `recipient => sender => subshare`.
    let received_subshares = new_signer_idents
        .into_iter()
        .map(|recipient_id| {
            let received_subshares = all_subshares
                .iter()
                .map(|(&sender_id, sender_shares)| {
                    (*sender_id, sender_shares[&recipient_id].clone())
                })
                .collect::<BTreeMap<_, _>>();
            (recipient_id, received_subshares)
        })
        .collect::<BTreeMap<_, BTreeMap<_, SecretSubshare>>>();

    // Recipients of the resharing can now validate and compute their new shares.

    let (new_seckeys_1, new_pubkeys_1) = reshare_step_2(
        *helper_1.identifier(),
        &old_pubkeys,
        min_signers,
        new_signer_idents.as_slice(),
        &received_subshares[&helper_1.identifier()],
    )
    .expect("error computing reshared share for signer 1");

    let (new_seckeys_2, new_pubkeys_2) = reshare_step_2(
        *helper_2.identifier(),
        &old_pubkeys,
        min_signers,
        &new_signer_idents,
        &received_subshares[&helper_2.identifier()],
    )
    .expect("error computing reshared share for signer 2");

    let (new_seckeys_4, new_pubkeys_4) = reshare_step_2(
        *helper_4.identifier(),
        &old_pubkeys,
        min_signers,
        &new_signer_idents,
        &received_subshares[&helper_4.identifier()],
    )
    .expect("error computing reshared share for signer 4");

    // all signers should compute the same group pubkeys.
    assert_eq!(new_pubkeys_1, new_pubkeys_2);
    assert_eq!(new_pubkeys_1, new_pubkeys_4);
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_2.verifying_key());
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_4.verifying_key());

    // The new pubkey package should be the same group key as the old one,
    // but with new coefficients and shares.
    assert_eq!(new_pubkeys_1.verifying_key(), old_pubkeys.verifying_key());
    assert_ne!(
        new_pubkeys_1.verifying_shares(),
        old_pubkeys.verifying_shares()
    );

    assert_eq!(new_seckeys_1.min_signers(), &min_signers);
}

/// Initial Min Signers: 3, Max Signers: 5
/// New     Min Signers: 3, Max Signers: 5
/// Signers 1, 2, 4 will participate in resharing.
/// They will reshare the key amongst themselves, plus new signer 5.
/// Signer 3 will be excluded.
/// The min signers threshold won't be changed.
/// Each helper generates their random coefficients and commitments.
/// All signers should compute the same group pubkeys.
/// The new pubkey package should be the same group key as the old one,
/// but with new coefficients and shares.
#[test]
fn reshare_verify_key_7() {
    let mut rng = thread_rng();

    let max_signers = 5;
    let min_signers = 3;
    let (old_shares, old_pubkeys) = gen_keys(min_signers, max_signers, &mut rng);

    // Signer 1, 2, and 4 will participate in resharing.
    let helper_1 = &old_shares[&Identifier::try_from(1).unwrap()];
    let helper_2 = &old_shares[&Identifier::try_from(2).unwrap()];
    let helper_4 = &old_shares[&Identifier::try_from(4).unwrap()];

    // They will reshare the key amongst themselves, plus new signer 5.
    // Signer 3 will be excluded.
    let new_signer_5_ident = Identifier::try_from(5).unwrap();
    let new_signer_idents = [
        *helper_1.identifier(),
        *helper_2.identifier(),
        *helper_4.identifier(),
        new_signer_5_ident,
    ];

    // The threshold will be changed from 3 to 3.
    let min_signers = 3;

    // Each helper generates their random coefficients and commitments.
    let helper_1_subshares = reshare_step_1(
        helper_1.signing_share(),
        &mut rng,
        min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 1");

    let helper_2_subshares = reshare_step_1(
        helper_2.signing_share(),
        &mut rng,
        min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 2");

    let helper_4_subshares = reshare_step_1(
        helper_4.signing_share(),
        &mut rng,
        min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 4");

    let all_subshares = BTreeMap::from([
        (helper_1.identifier(), helper_1_subshares),
        (helper_2.identifier(), helper_2_subshares),
        (helper_4.identifier(), helper_4_subshares),
    ]);

    // Sort the subshares into a map of `recipient => sender => subshare`.
    let received_subshares = new_signer_idents
        .into_iter()
        .map(|recipient_id| {
            let received_subshares = all_subshares
                .iter()
                .map(|(&sender_id, sender_shares)| {
                    (*sender_id, sender_shares[&recipient_id].clone())
                })
                .collect::<BTreeMap<_, _>>();
            (recipient_id, received_subshares)
        })
        .collect::<BTreeMap<_, BTreeMap<_, SecretSubshare>>>();

    // Recipients of the resharing can now validate and compute their new shares.

    let (new_seckeys_1, new_pubkeys_1) = reshare_step_2(
        *helper_1.identifier(),
        &old_pubkeys,
        min_signers,
        new_signer_idents.as_slice(),
        &received_subshares[&helper_1.identifier()],
    )
    .expect("error computing reshared share for signer 1");

    let (new_seckeys_2, new_pubkeys_2) = reshare_step_2(
        *helper_2.identifier(),
        &old_pubkeys,
        min_signers,
        &new_signer_idents,
        &received_subshares[&helper_2.identifier()],
    )
    .expect("error computing reshared share for signer 2");

    let (new_seckeys_4, new_pubkeys_4) = reshare_step_2(
        *helper_4.identifier(),
        &old_pubkeys,
        min_signers,
        &new_signer_idents,
        &received_subshares[&helper_4.identifier()],
    )
    .expect("error computing reshared share for signer 4");

    let (new_seckeys_5, new_pubkeys_5) = reshare_step_2(
        new_signer_5_ident,
        &old_pubkeys,
        min_signers,
        &new_signer_idents,
        &received_subshares[&new_signer_5_ident],
    )
    .expect("error computing reshared share for signer 5");

    // all signers should compute the same group pubkeys.
    assert_eq!(new_pubkeys_1, new_pubkeys_2);
    assert_eq!(new_pubkeys_1, new_pubkeys_4);
    assert_eq!(new_pubkeys_1, new_pubkeys_5);
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_2.verifying_key());
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_4.verifying_key());
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_5.verifying_key());

    // The new pubkey package should be the same group key as the old one,
    // but with new coefficients and shares.
    assert_eq!(new_pubkeys_1.verifying_key(), old_pubkeys.verifying_key());
    assert_ne!(
        new_pubkeys_1.verifying_shares(),
        old_pubkeys.verifying_shares()
    );

    assert_eq!(new_seckeys_1.min_signers(), &min_signers);
}

/// Initial Min Signers: 3, Max Signers: 5
/// New     Min Signers: 4, Max Signers: 5
/// Signers 1, 2, 4 will participate in resharing.
/// They will reshare the key amongst themselves, plus new signers 5 and 6.
/// Signer 3 will be excluded.
/// The threshold will be changed from 3 to 4.
/// Each helper generates their random coefficients and commitments.
/// All signers should compute the same group pubkeys.
/// The new pubkey package should be the same group key as the old one,
/// but with new coefficients and shares.
#[test]
fn reshare_verify_key_8() {
    let mut rng = thread_rng();

    let max_signers = 5;
    let old_min_signers = 3;
    let (old_shares, old_pubkeys) = gen_keys(old_min_signers, max_signers, &mut rng);

    // Signer 1, 2, and 4 will participate in resharing.
    let helper_1 = &old_shares[&Identifier::try_from(1).unwrap()];
    let helper_2 = &old_shares[&Identifier::try_from(2).unwrap()];
    let helper_4 = &old_shares[&Identifier::try_from(4).unwrap()];

    // They will reshare the key amongst themselves, plus new signers 5 and 6.
    // Signer 3 will be excluded.
    let new_signer_5_ident = Identifier::try_from(5).unwrap();
    let new_signer_6_ident = Identifier::try_from(6).unwrap();
    let new_signer_idents = [
        *helper_1.identifier(),
        *helper_2.identifier(),
        *helper_4.identifier(),
        new_signer_5_ident,
        new_signer_6_ident,
    ];

    // The threshold will be changed from 3 to 4.
    let new_min_signers = 4;

    // Each helper generates their random coefficients and commitments.
    let helper_1_subshares = reshare_step_1(
        helper_1.signing_share(),
        &mut rng,
        new_min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 1");

    let helper_2_subshares = reshare_step_1(
        helper_2.signing_share(),
        &mut rng,
        new_min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 2");

    let helper_4_subshares = reshare_step_1(
        helper_4.signing_share(),
        &mut rng,
        new_min_signers,
        &new_signer_idents,
    )
    .expect("error computing resharing step 1 for helper 4");

    let all_subshares = BTreeMap::from([
        (helper_1.identifier(), helper_1_subshares),
        (helper_2.identifier(), helper_2_subshares),
        (helper_4.identifier(), helper_4_subshares),
    ]);

    // Sort the subshares into a map of `recipient => sender => subshare`.
    let received_subshares = new_signer_idents
        .into_iter()
        .map(|recipient_id| {
            let received_subshares = all_subshares
                .iter()
                .map(|(&sender_id, sender_shares)| {
                    (*sender_id, sender_shares[&recipient_id].clone())
                })
                .collect::<BTreeMap<_, _>>();
            (recipient_id, received_subshares)
        })
        .collect::<BTreeMap<_, BTreeMap<_, SecretSubshare>>>();

    // Recipients of the resharing can now validate and compute their new shares.

    let (new_seckeys_1, new_pubkeys_1) = reshare_step_2(
        *helper_1.identifier(),
        &old_pubkeys,
        new_min_signers,
        new_signer_idents.as_slice(),
        &received_subshares[&helper_1.identifier()],
    )
    .expect("error computing reshared share for signer 1");

    let (new_seckeys_2, new_pubkeys_2) = reshare_step_2(
        *helper_2.identifier(),
        &old_pubkeys,
        new_min_signers,
        &new_signer_idents,
        &received_subshares[&helper_2.identifier()],
    )
    .expect("error computing reshared share for signer 2");

    let (new_seckeys_4, new_pubkeys_4) = reshare_step_2(
        *helper_4.identifier(),
        &old_pubkeys,
        new_min_signers,
        &new_signer_idents,
        &received_subshares[&helper_4.identifier()],
    )
    .expect("error computing reshared share for signer 4");

    let (new_seckeys_5, new_pubkeys_5) = reshare_step_2(
        new_signer_5_ident,
        &old_pubkeys,
        new_min_signers,
        &new_signer_idents,
        &received_subshares[&new_signer_5_ident],
    )
    .expect("error computing reshared share for signer 5");

    let (new_seckeys_6, new_pubkeys_6) = reshare_step_2(
        new_signer_6_ident,
        &old_pubkeys,
        new_min_signers,
        &new_signer_idents,
        &received_subshares[&new_signer_6_ident],
    )
    .expect("error computing reshared share for signer 6");

    // all signers should compute the same group pubkeys.
    assert_eq!(new_pubkeys_1, new_pubkeys_2);
    assert_eq!(new_pubkeys_1, new_pubkeys_4);
    assert_eq!(new_pubkeys_1, new_pubkeys_5);
    assert_eq!(new_pubkeys_1, new_pubkeys_6);
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_2.verifying_key());
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_4.verifying_key());
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_5.verifying_key());
    assert_eq!(new_seckeys_1.verifying_key(), new_seckeys_6.verifying_key());

    // The new pubkey package should be the same group key as the old one,
    // but with new coefficients and shares.
    assert_eq!(new_pubkeys_1.verifying_key(), old_pubkeys.verifying_key());
    assert_ne!(
        new_pubkeys_1.verifying_shares(),
        old_pubkeys.verifying_shares()
    );

    assert_eq!(new_seckeys_1.min_signers(), &new_min_signers);
}
