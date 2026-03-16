//! Integration tests for the encrypted DM messaging pipeline.
//!
//! These tests exercise the **complete end-to-end flow** that two clients
//! would perform to exchange encrypted direct messages, from X3DH key
//! agreement through Double Ratchet message encryption and the wire format
//! used by the API.
//!
//! The pipeline under test:
//! 1. Both users generate identity keys, signed pre-keys, and one-time pre-keys
//! 2. Key material is serialized in the hex format used by the `/api/keys/` endpoints
//! 3. The initiator fetches the responder's pre-key bundle
//! 4. X3DH produces a shared secret on both sides
//! 5. The Double Ratchet is initialized with that shared secret
//! 6. Messages are encrypted, serialized, and decrypted as they would be
//!    when sent through `/api/dm/` (hex-encoded ciphertext + nonce)

use src_backend::crypto::double_ratchet::{DoubleRatchet, RatchetMessage};
use src_backend::crypto::x3dh::{
    IdentityKeyPair, OneTimePreKeyPair, PreKeyBundle, SignedPreKeyPair, x3dh_initiate, x3dh_respond,
};

// Helper: Simulates what the server stores / returns for a pre-key bundle.

/// Hex-encoded representation of a user's published key material,
/// matching the `PreKeyBundleResponse` returned by `GET /api/keys/bundle/{user_id}`.
struct HexBundle {
    identity_key: String,
    signed_prekey: String,
    #[expect(dead_code)]
    signed_prekey_signature: String,
    one_time_prekey: Option<String>,
}

/// Build a hex-encoded bundle from raw key pairs, as the server would store it.
fn bundle_to_hex(
    identity: &IdentityKeyPair,
    signed: &SignedPreKeyPair,
    one_time: Option<&OneTimePreKeyPair>,
) -> HexBundle {
    HexBundle {
        identity_key: hex::encode(identity.public.as_bytes()),
        signed_prekey: hex::encode(signed.public.as_bytes()),
        // In a real implementation the signature would be an Ed25519 signature;
        // for this test we use a placeholder since signature verification is
        // not performed during the X3DH key agreement itself.
        signed_prekey_signature: hex::encode([0xAA; 64]),
        one_time_prekey: one_time.map(|otpk| hex::encode(otpk.public.as_bytes())),
    }
}

/// Parse a hex-encoded bundle back into a `PreKeyBundle` of public keys,
/// simulating what a client does after calling `GET /api/keys/bundle/{user_id}`.
fn hex_to_prekey_bundle(hex_bundle: &HexBundle) -> PreKeyBundle {
    let identity_bytes: [u8; 32] = hex::decode(&hex_bundle.identity_key)
        .expect("valid hex")
        .try_into()
        .expect("32 bytes");
    let signed_bytes: [u8; 32] = hex::decode(&hex_bundle.signed_prekey)
        .expect("valid hex")
        .try_into()
        .expect("32 bytes");
    let otpk = hex_bundle.one_time_prekey.as_ref().map(|h| {
        let bytes: [u8; 32] = hex::decode(h)
            .expect("valid hex")
            .try_into()
            .expect("32 bytes");
        x25519_dalek::PublicKey::from(bytes)
    });

    PreKeyBundle {
        identity_key: x25519_dalek::PublicKey::from(identity_bytes),
        signed_prekey: x25519_dalek::PublicKey::from(signed_bytes),
        one_time_prekey: otpk,
    }
}

// Helper: Simulates the wire format used by POST /api/dm/{channel_id}/messages

/// What a client would send in `NewDirectMessage` JSON body.
struct WireMessage {
    ciphertext: String, // Hex-encoded.
    nonce: String,      // Hex-encoded, 24 hex chars (12 bytes).
    #[expect(dead_code)]
    ratchet_key_id: i64,
}

/// Convert a `RatchetMessage` to the hex-encoded wire format the API expects.
fn ratchet_msg_to_wire(msg: &RatchetMessage, ratchet_key_id: i64) -> WireMessage {
    WireMessage {
        ciphertext: hex::encode(&msg.ciphertext),
        nonce: hex::encode(msg.nonce),
        ratchet_key_id,
    }
}

/// Reconstruct a `RatchetMessage` from the wire format (simulating what a
/// client does after `GET /api/dm/{channel_id}/messages`).
fn wire_to_ratchet_msg(
    wire: &WireMessage,
    ratchet_pub: [u8; 32],
    prev_chain_len: u32,
    msg_num: u32,
) -> RatchetMessage {
    RatchetMessage {
        ratchet_pub,
        prev_chain_len,
        msg_num,
        ciphertext: hex::decode(&wire.ciphertext).expect("valid hex"),
        nonce: hex::decode(&wire.nonce)
            .expect("valid hex")
            .try_into()
            .expect("12 bytes"),
    }
}

/// Full end-to-end: X3DH key agreement -> Double Ratchet init -> encrypted
/// message exchange, using the hex wire format that the API endpoints expect.
#[test]
fn full_x3dh_to_double_ratchet_dm_flow() {
    // 1. Key generation (both users).
    let alice_identity = IdentityKeyPair::generate();
    let bob_identity = IdentityKeyPair::generate();
    let bob_signed_prekey = SignedPreKeyPair::generate();
    let bob_one_time = OneTimePreKeyPair::generate(1);

    // 2. Bob publishes his key bundle (hex-encoded, as stored on server).
    let hex_bundle = bundle_to_hex(&bob_identity, &bob_signed_prekey, Some(&bob_one_time));

    // Verify the hex encoding produces the expected lengths.
    assert_eq!(
        hex_bundle.identity_key.len(),
        64,
        "identity key = 32 bytes = 64 hex chars"
    );
    assert_eq!(hex_bundle.signed_prekey.len(), 64);
    assert!(
        hex_bundle
            .one_time_prekey
            .as_ref()
            .is_some_and(|k| k.len() == 64)
    );

    // 3. Alice fetches Bob's bundle and parses it.
    let bundle = hex_to_prekey_bundle(&hex_bundle);

    // 4. X3DH key agreement.
    let alice_x3dh = x3dh_initiate(&alice_identity, &bundle);
    let bob_shared_secret = x3dh_respond(
        &bob_identity,
        &bob_signed_prekey,
        Some(&bob_one_time),
        &alice_identity.public,
        &alice_x3dh.ephemeral_public,
    );
    assert_eq!(
        alice_x3dh.shared_secret, bob_shared_secret,
        "X3DH must produce identical shared secrets"
    );

    // 5. Double Ratchet initialization.
    let mut alice_ratchet =
        DoubleRatchet::init_alice(alice_x3dh.shared_secret, bob_signed_prekey.public);
    let mut bob_ratchet = DoubleRatchet::init_bob(bob_shared_secret, bob_signed_prekey.secret);

    // 6. Alice sends an encrypted DM to Bob.
    let plaintext = b"Hey Bob, this is a secret message!";
    let encrypted = alice_ratchet.encrypt(plaintext);

    // Convert to wire format (what POST /api/dm/.../messages receives).
    let wire = ratchet_msg_to_wire(&encrypted, 1);

    // Verify nonce is exactly 12 bytes (24 hex chars) as the API requires.
    assert_eq!(
        wire.nonce.len(),
        24,
        "nonce must be 24 hex chars (12 bytes)"
    );

    // 7. Bob receives and decrypts.
    // In practice Bob would reconstruct the RatchetMessage from the stored
    // ciphertext + nonce + the ratchet_pub sent as part of the message header.
    let reconstructed = wire_to_ratchet_msg(
        &wire,
        encrypted.ratchet_pub,
        encrypted.prev_chain_len,
        encrypted.msg_num,
    );
    let decrypted = bob_ratchet
        .decrypt(&reconstructed)
        .expect("Bob should decrypt Alice's message");
    assert_eq!(decrypted, plaintext);

    // 8. Bob replies.
    let reply_plain = b"Got it, Alice! Replying securely.";
    let reply_encrypted = bob_ratchet.encrypt(reply_plain);
    let reply_wire = ratchet_msg_to_wire(&reply_encrypted, 2);
    assert_eq!(reply_wire.nonce.len(), 24);

    let reply_reconstructed = wire_to_ratchet_msg(
        &reply_wire,
        reply_encrypted.ratchet_pub,
        reply_encrypted.prev_chain_len,
        reply_encrypted.msg_num,
    );
    let reply_decrypted = alice_ratchet
        .decrypt(&reply_reconstructed)
        .expect("Alice should decrypt Bob's reply");
    assert_eq!(reply_decrypted, reply_plain);
}

/// Simulates a realistic multi-message conversation where both parties
/// alternate sending messages, exercising multiple DH ratchet steps.
#[test]
fn multi_round_conversation_through_wire_format() {
    let alice_identity = IdentityKeyPair::generate();
    let bob_identity = IdentityKeyPair::generate();
    let bob_spk = SignedPreKeyPair::generate();
    let bob_otpk = OneTimePreKeyPair::generate(42);

    let bundle = PreKeyBundle {
        identity_key: bob_identity.public,
        signed_prekey: bob_spk.public,
        one_time_prekey: Some(bob_otpk.public),
    };

    let alice_x3dh = x3dh_initiate(&alice_identity, &bundle);
    let bob_secret = x3dh_respond(
        &bob_identity,
        &bob_spk,
        Some(&bob_otpk),
        &alice_identity.public,
        &alice_x3dh.ephemeral_public,
    );

    let mut alice = DoubleRatchet::init_alice(alice_x3dh.shared_secret, bob_spk.public);
    let mut bob = DoubleRatchet::init_bob(bob_secret, bob_spk.secret);

    // 20 rounds of alternating messages.
    for round in 0u32..20 {
        let a_text = format!("Alice message #{round}");
        let a_msg = alice.encrypt(a_text.as_bytes());
        let a_wire = ratchet_msg_to_wire(&a_msg, i64::from(round) * 2);

        // Verify wire format invariants.
        assert_eq!(a_wire.nonce.len(), 24);
        assert!(!a_wire.ciphertext.is_empty());

        let a_reconstructed = wire_to_ratchet_msg(
            &a_wire,
            a_msg.ratchet_pub,
            a_msg.prev_chain_len,
            a_msg.msg_num,
        );
        let a_decrypted = bob.decrypt(&a_reconstructed).expect("Bob decrypts");
        assert_eq!(a_decrypted, a_text.as_bytes());

        let b_text = format!("Bob message #{round}");
        let b_msg = bob.encrypt(b_text.as_bytes());
        let b_wire = ratchet_msg_to_wire(&b_msg, i64::from(round) * 2 + 1);

        let b_reconstructed = wire_to_ratchet_msg(
            &b_wire,
            b_msg.ratchet_pub,
            b_msg.prev_chain_len,
            b_msg.msg_num,
        );
        let b_decrypted = alice.decrypt(&b_reconstructed).expect("Alice decrypts");
        assert_eq!(b_decrypted, b_text.as_bytes());
    }
}

/// Verify that the hex bundle round-trip preserves all key material exactly.
#[test]
fn prekey_bundle_hex_round_trip() {
    let identity = IdentityKeyPair::generate();
    let signed = SignedPreKeyPair::generate();
    let otpk = OneTimePreKeyPair::generate(7);

    let hex_bundle = bundle_to_hex(&identity, &signed, Some(&otpk));
    let parsed = hex_to_prekey_bundle(&hex_bundle);

    assert_eq!(parsed.identity_key.as_bytes(), identity.public.as_bytes());
    assert_eq!(parsed.signed_prekey.as_bytes(), signed.public.as_bytes());
    assert_eq!(
        parsed
            .one_time_prekey
            .as_ref()
            .map(x25519_dalek::PublicKey::as_bytes),
        Some(otpk.public.as_bytes())
    );
}

/// Verify round-trip without a one-time prekey (the optional case).
#[test]
fn prekey_bundle_hex_round_trip_without_otpk() {
    let identity = IdentityKeyPair::generate();
    let signed = SignedPreKeyPair::generate();

    let hex_bundle = bundle_to_hex(&identity, &signed, None);
    assert!(hex_bundle.one_time_prekey.is_none());

    let parsed = hex_to_prekey_bundle(&hex_bundle);
    assert!(parsed.one_time_prekey.is_none());
    assert_eq!(parsed.identity_key.as_bytes(), identity.public.as_bytes());
}

/// After consuming a one-time prekey the next bundle fetch should not include
/// it, matching the server's `DELETE` on `GET /api/keys/bundle/{user_id}`.
#[test]
fn one_time_prekey_consumption_simulation() {
    let alice_identity = IdentityKeyPair::generate();
    let bob_identity = IdentityKeyPair::generate();
    let bob_spk = SignedPreKeyPair::generate();

    // Bob uploads 3 one-time prekeys.
    let otpk1 = OneTimePreKeyPair::generate(1);
    let otpk2 = OneTimePreKeyPair::generate(2);
    let otpk3 = OneTimePreKeyPair::generate(3);

    // Simulate server returning oldest first and deleting after fetch.
    let mut available_otpks = vec![&otpk1, &otpk2, &otpk3];

    // First fetch: returns otpk1.
    let consumed = available_otpks.remove(0);
    let bundle1 = PreKeyBundle {
        identity_key: bob_identity.public,
        signed_prekey: bob_spk.public,
        one_time_prekey: Some(consumed.public),
    };
    let result1 = x3dh_initiate(&alice_identity, &bundle1);
    let secret1 = x3dh_respond(
        &bob_identity,
        &bob_spk,
        Some(consumed),
        &alice_identity.public,
        &result1.ephemeral_public,
    );
    assert_eq!(result1.shared_secret, secret1);

    // Second fetch: returns otpk2 (otpk1 is gone).
    let consumed = available_otpks.remove(0);
    let bundle2 = PreKeyBundle {
        identity_key: bob_identity.public,
        signed_prekey: bob_spk.public,
        one_time_prekey: Some(consumed.public),
    };
    let result2 = x3dh_initiate(&alice_identity, &bundle2);
    let secret2 = x3dh_respond(
        &bob_identity,
        &bob_spk,
        Some(consumed),
        &alice_identity.public,
        &result2.ephemeral_public,
    );
    assert_eq!(result2.shared_secret, secret2);

    // Each session should produce a different shared secret (different ephemeral keys).
    assert_ne!(result1.shared_secret, result2.shared_secret);

    // Third fetch: returns otpk3.
    assert_eq!(available_otpks.len(), 1);
    let consumed = available_otpks.remove(0);
    assert_eq!(consumed.id, 3);
}

/// When no one-time prekeys are available, X3DH still works (without DH4).
/// This tests the graceful fallback path.
#[test]
fn x3dh_session_without_one_time_prekey() {
    let alice_identity = IdentityKeyPair::generate();
    let bob_identity = IdentityKeyPair::generate();
    let bob_spk = SignedPreKeyPair::generate();

    let bundle = PreKeyBundle {
        identity_key: bob_identity.public,
        signed_prekey: bob_spk.public,
        one_time_prekey: None,
    };

    let alice_x3dh = x3dh_initiate(&alice_identity, &bundle);
    let bob_secret = x3dh_respond(
        &bob_identity,
        &bob_spk,
        None,
        &alice_identity.public,
        &alice_x3dh.ephemeral_public,
    );

    assert_eq!(alice_x3dh.shared_secret, bob_secret);

    // Initialize ratchets and verify messaging works.
    let mut alice = DoubleRatchet::init_alice(alice_x3dh.shared_secret, bob_spk.public);
    let mut bob = DoubleRatchet::init_bob(bob_secret, bob_spk.secret);

    let msg = alice.encrypt(b"no OTP, still secure");
    let pt = bob.decrypt(&msg).expect("decrypt without OTP");
    assert_eq!(pt, b"no OTP, still secure");
}

/// Test that the `RatchetMessage` struct round-trips through serde JSON
/// serialization, which is how it would travel over WebSocket.
#[test]
fn ratchet_message_json_serialization() {
    let alice_identity = IdentityKeyPair::generate();
    let bob_identity = IdentityKeyPair::generate();
    let bob_spk = SignedPreKeyPair::generate();

    let bundle = PreKeyBundle {
        identity_key: bob_identity.public,
        signed_prekey: bob_spk.public,
        one_time_prekey: None,
    };

    let alice_x3dh = x3dh_initiate(&alice_identity, &bundle);
    let bob_secret = x3dh_respond(
        &bob_identity,
        &bob_spk,
        None,
        &alice_identity.public,
        &alice_x3dh.ephemeral_public,
    );

    let mut alice = DoubleRatchet::init_alice(alice_x3dh.shared_secret, bob_spk.public);
    let mut bob = DoubleRatchet::init_bob(bob_secret, bob_spk.secret);

    let msg = alice.encrypt(b"JSON round-trip test");

    // Serialize to JSON and back.
    let json = serde_json::to_string(&msg).expect("serialize");
    let deserialized: RatchetMessage = serde_json::from_str(&json).expect("deserialize");

    // Verify fields are preserved.
    assert_eq!(deserialized.ratchet_pub, msg.ratchet_pub);
    assert_eq!(deserialized.prev_chain_len, msg.prev_chain_len);
    assert_eq!(deserialized.msg_num, msg.msg_num);
    assert_eq!(deserialized.ciphertext, msg.ciphertext);
    assert_eq!(deserialized.nonce, msg.nonce);

    // Bob can decrypt the deserialized message.
    let pt = bob.decrypt(&deserialized).expect("decrypt deserialized");
    assert_eq!(pt, b"JSON round-trip test");
}

/// Verify that wire format ciphertext cannot be tampered with - any
/// modification must be detected by AES-GCM authentication.
#[test]
fn wire_format_tamper_detection() {
    let alice_identity = IdentityKeyPair::generate();
    let bob_identity = IdentityKeyPair::generate();
    let bob_spk = SignedPreKeyPair::generate();

    let bundle = PreKeyBundle {
        identity_key: bob_identity.public,
        signed_prekey: bob_spk.public,
        one_time_prekey: None,
    };

    let alice_x3dh = x3dh_initiate(&alice_identity, &bundle);
    let bob_secret = x3dh_respond(
        &bob_identity,
        &bob_spk,
        None,
        &alice_identity.public,
        &alice_x3dh.ephemeral_public,
    );

    let mut alice = DoubleRatchet::init_alice(alice_x3dh.shared_secret, bob_spk.public);
    let mut bob = DoubleRatchet::init_bob(bob_secret, bob_spk.secret);

    let msg = alice.encrypt(b"authentic message");
    let wire = ratchet_msg_to_wire(&msg, 1);

    // Tamper with the hex-encoded ciphertext (flip the first byte).
    let mut tampered_hex = wire.ciphertext;
    let first_byte = u8::from_str_radix(&tampered_hex[..2], 16).expect("parse hex byte");
    let flipped = first_byte ^ 0xFF;
    tampered_hex.replace_range(..2, &format!("{flipped:02x}"));

    let tampered = RatchetMessage {
        ratchet_pub: msg.ratchet_pub,
        prev_chain_len: msg.prev_chain_len,
        msg_num: msg.msg_num,
        ciphertext: hex::decode(&tampered_hex).expect("valid hex"),
        nonce: msg.nonce,
    };

    assert!(
        bob.decrypt(&tampered).is_err(),
        "Tampered ciphertext must fail AES-GCM authentication"
    );
}

/// Simulate two independent DM sessions between Alice -> Bob and Alice ->
/// Carol, verifying that messages from one session cannot be decrypted in the
/// other.
///
/// Note: a failed `decrypt()` call corrupts the ratchet state (it performs an
/// irreversible DH ratchet step with the wrong key), so cross-session decrypt
/// attempts are done *after* all legitimate operations on that ratchet.
#[test]
fn independent_sessions_are_isolated() {
    let alice_identity = IdentityKeyPair::generate();

    // Bob's keys
    let bob_identity = IdentityKeyPair::generate();
    let bob_spk = SignedPreKeyPair::generate();

    // Carol's keys
    let carol_identity = IdentityKeyPair::generate();
    let carol_spk = SignedPreKeyPair::generate();

    // Session with Bob
    let bob_bundle = PreKeyBundle {
        identity_key: bob_identity.public,
        signed_prekey: bob_spk.public,
        one_time_prekey: None,
    };
    let alice_bob_x3dh = x3dh_initiate(&alice_identity, &bob_bundle);
    let bob_secret = x3dh_respond(
        &bob_identity,
        &bob_spk,
        None,
        &alice_identity.public,
        &alice_bob_x3dh.ephemeral_public,
    );
    let mut alice_to_bob = DoubleRatchet::init_alice(alice_bob_x3dh.shared_secret, bob_spk.public);
    let mut bob_ratchet = DoubleRatchet::init_bob(bob_secret, bob_spk.secret);

    // Session with Carol.
    let carol_bundle = PreKeyBundle {
        identity_key: carol_identity.public,
        signed_prekey: carol_spk.public,
        one_time_prekey: None,
    };

    let alice_carol_x3dh = x3dh_initiate(&alice_identity, &carol_bundle);
    let carol_secret = x3dh_respond(
        &carol_identity,
        &carol_spk,
        None,
        &alice_identity.public,
        &alice_carol_x3dh.ephemeral_public,
    );
    let mut alice_to_carol =
        DoubleRatchet::init_alice(alice_carol_x3dh.shared_secret, carol_spk.public);
    let mut carol_ratchet = DoubleRatchet::init_bob(carol_secret, carol_spk.secret);

    // Shared secrets must differ.
    assert_ne!(alice_bob_x3dh.shared_secret, alice_carol_x3dh.shared_secret);

    // Send a message to Bob - Bob decrypts successfully.
    let msg_for_bob = alice_to_bob.encrypt(b"Hello Bob");
    let bob_pt = bob_ratchet.decrypt(&msg_for_bob).expect("Bob decrypts");
    assert_eq!(bob_pt, b"Hello Bob");

    // Send a message to Carol - Carol decrypts successfully.
    let msg_for_carol = alice_to_carol.encrypt(b"Hello Carol");
    let carol_pt = carol_ratchet
        .decrypt(&msg_for_carol)
        .expect("Carol decrypts");
    assert_eq!(carol_pt, b"Hello Carol");

    // Now test cross-session isolation (these corrupt the ratchet state, so
    // they must be the last operations on each ratchet).
    assert!(
        carol_ratchet.decrypt(&msg_for_bob).is_err(),
        "Carol must not decrypt a message intended for Bob"
    );
    assert!(
        bob_ratchet.decrypt(&msg_for_carol).is_err(),
        "Bob must not decrypt a message intended for Carol"
    );
}

/// Test burst of messages in one direction followed by a reply - a common
/// pattern in real chat (someone sends several messages before the other
/// person responds).
#[test]
fn burst_messages_then_reply() {
    let alice_identity = IdentityKeyPair::generate();
    let bob_identity = IdentityKeyPair::generate();
    let bob_spk = SignedPreKeyPair::generate();

    let bundle = PreKeyBundle {
        identity_key: bob_identity.public,
        signed_prekey: bob_spk.public,
        one_time_prekey: None,
    };

    let alice_x3dh = x3dh_initiate(&alice_identity, &bundle);
    let bob_secret = x3dh_respond(
        &bob_identity,
        &bob_spk,
        None,
        &alice_identity.public,
        &alice_x3dh.ephemeral_public,
    );

    let mut alice = DoubleRatchet::init_alice(alice_x3dh.shared_secret, bob_spk.public);
    let mut bob = DoubleRatchet::init_bob(bob_secret, bob_spk.secret);

    // Alice sends 10 messages in a burst.
    let mut alice_msgs = Vec::new();
    for i in 0..10 {
        let msg = alice.encrypt(format!("burst {i}").as_bytes());
        alice_msgs.push(msg);
    }

    // Bob receives all 10.
    for (i, msg) in alice_msgs.iter().enumerate() {
        let pt = bob.decrypt(msg).expect("Bob decrypts burst");
        assert_eq!(pt, format!("burst {i}").as_bytes());
    }

    // Bob replies with a single message (triggers DH ratchet step).
    let reply = bob.encrypt(b"got all 10!");
    let pt = alice.decrypt(&reply).expect("Alice decrypts reply");
    assert_eq!(pt, b"got all 10!");

    // Alice sends another burst after the ratchet turn.
    for i in 0..5 {
        let msg = alice.encrypt(format!("second burst {i}").as_bytes());
        let pt = bob.decrypt(&msg).expect("Bob decrypts second burst");
        assert_eq!(pt, format!("second burst {i}").as_bytes());
    }
}

/// Ensure that each encrypted message produces unique ciphertext, even for
/// identical plaintext - this is critical for preventing frequency analysis.
#[test]
fn identical_plaintexts_produce_different_ciphertexts() {
    let alice_identity = IdentityKeyPair::generate();
    let bob_identity = IdentityKeyPair::generate();
    let bob_spk = SignedPreKeyPair::generate();

    let bundle = PreKeyBundle {
        identity_key: bob_identity.public,
        signed_prekey: bob_spk.public,
        one_time_prekey: None,
    };

    let alice_x3dh = x3dh_initiate(&alice_identity, &bundle);

    let mut alice = DoubleRatchet::init_alice(alice_x3dh.shared_secret, bob_spk.public);

    let msg1 = alice.encrypt(b"same plaintext");
    let msg2 = alice.encrypt(b"same plaintext");

    // Both ciphertext and nonce must differ (due to chain ratchet + random nonce).
    assert_ne!(msg1.ciphertext, msg2.ciphertext);
    assert_ne!(msg1.nonce, msg2.nonce);
}
