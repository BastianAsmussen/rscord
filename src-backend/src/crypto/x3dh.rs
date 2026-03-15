//! Extended Triple Diffie-Hellman (X3DH) key agreement protocol.
//!
//! Implements the X3DH specification used by the Signal Protocol for
//! establishing a shared secret between two parties, even when the
//! recipient is offline.
//!
//! Reference: <https://signal.org/docs/specifications/x3dh/>

use hkdf::Hkdf;
use rand_core_06::OsRng;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

/// A long-lived identity key pair (Curve25519).
pub struct IdentityKeyPair {
    pub secret: StaticSecret,
    pub public: PublicKey,
}

impl IdentityKeyPair {
    /// Generate a new random identity key pair.
    #[must_use]
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }
}

/// A signed pre-key pair.
///
/// In a full implementation the public half would be
/// signed by the identity key (Ed25519). We store it as a Curve25519 key pair
/// here; signature verification is handled at the API layer.
pub struct SignedPreKeyPair {
    pub secret: StaticSecret,
    pub public: PublicKey,
}

impl SignedPreKeyPair {
    /// Generate a new random signed pre-key pair.
    #[must_use]
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }
}

/// A one-time pre-key pair.
pub struct OneTimePreKeyPair {
    pub id: u64,
    pub secret: StaticSecret,
    pub public: PublicKey,
}

impl OneTimePreKeyPair {
    /// Generate a new random one-time pre-key pair with the given id.
    #[must_use]
    pub fn generate(id: u64) -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { id, secret, public }
    }
}

/// A pre-key bundle published by a user so others can initiate sessions.
/// Contains only public keys.
pub struct PreKeyBundle {
    pub identity_key: PublicKey,
    pub signed_prekey: PublicKey,
    /// Optional one-time pre-key. If available, provides extra forward secrecy.
    pub one_time_prekey: Option<PublicKey>,
}

/// The result of the initiator (Alice) side of X3DH.
pub struct X3dhInitiatorResult {
    /// The shared secret derived from X3DH, used as the initial root key
    /// for the Double Ratchet.
    pub shared_secret: [u8; 32],
    /// Alice's ephemeral public key, sent to Bob so he can compute the same
    /// shared secret.
    pub ephemeral_public: PublicKey,
}

/// Initiator (Alice) performs X3DH with Bob's pre-key bundle.
///
/// Computes: `DH1 = DH(IK_A, SPK_B)`, `DH2 = DH(EK_A, IK_B)`,
/// `DH3 = DH(EK_A, SPK_B)`, and optionally `DH4 = DH(EK_A, OPK_B)`.
///
/// The shared secret is derived via HKDF over the concatenation of all DH
/// outputs.
#[must_use]
pub fn x3dh_initiate(
    our_identity: &IdentityKeyPair,
    their_bundle: &PreKeyBundle,
) -> X3dhInitiatorResult {
    // We use StaticSecret here (not EphemeralSecret) because X3DH requires
    // multiple DH operations with the same ephemeral key, and EphemeralSecret
    // is consumed after a single diffie_hellman() call.
    let ephemeral_secret = StaticSecret::random_from_rng(OsRng);
    let ephemeral_public = PublicKey::from(&ephemeral_secret);

    // DH1 = DH(IK_A, SPK_B)
    let dh1 = our_identity
        .secret
        .diffie_hellman(&their_bundle.signed_prekey);
    // DH2 = DH(EK_A, IK_B)
    let dh2 = ephemeral_secret.diffie_hellman(&their_bundle.identity_key);
    // DH3 = DH(EK_A, SPK_B)
    let dh3 = ephemeral_secret.diffie_hellman(&their_bundle.signed_prekey);

    let mut dh_concat = Vec::with_capacity(128);
    dh_concat.extend_from_slice(dh1.as_bytes());
    dh_concat.extend_from_slice(dh2.as_bytes());
    dh_concat.extend_from_slice(dh3.as_bytes());

    // DH4 = DH(EK_A, OPK_B) (optional)
    if let Some(opk) = &their_bundle.one_time_prekey {
        let dh4 = ephemeral_secret.diffie_hellman(opk);
        dh_concat.extend_from_slice(dh4.as_bytes());
    }

    // Derive the shared secret with HKDF.
    // The info string includes protocol context as recommended by the spec.
    let shared_secret = derive_x3dh_secret(&dh_concat);

    X3dhInitiatorResult {
        shared_secret,
        ephemeral_public,
    }
}

/// Responder (Bob) performs X3DH with Alice's initial message.
///
/// Bob uses his own identity key, signed pre-key, and optionally a one-time
/// pre-key together with Alice's identity key and ephemeral key to derive the
/// same shared secret.
#[must_use]
pub fn x3dh_respond(
    our_identity: &IdentityKeyPair,
    our_signed_prekey: &SignedPreKeyPair,
    our_one_time_prekey: Option<&OneTimePreKeyPair>,
    their_identity: &PublicKey,
    their_ephemeral: &PublicKey,
) -> [u8; 32] {
    // DH1 = DH(SPK_B, IK_A)
    let dh1 = our_signed_prekey.secret.diffie_hellman(their_identity);
    // DH2 = DH(IK_B, EK_A)
    let dh2 = our_identity.secret.diffie_hellman(their_ephemeral);
    // DH3 = DH(SPK_B, EK_A)
    let dh3 = our_signed_prekey.secret.diffie_hellman(their_ephemeral);

    let mut dh_concat = Vec::with_capacity(128);
    dh_concat.extend_from_slice(dh1.as_bytes());
    dh_concat.extend_from_slice(dh2.as_bytes());
    dh_concat.extend_from_slice(dh3.as_bytes());

    // DH4 = DH(OPK_B, EK_A) (optional)
    if let Some(opk) = our_one_time_prekey {
        let dh4 = opk.secret.diffie_hellman(their_ephemeral);
        dh_concat.extend_from_slice(dh4.as_bytes());
    }

    derive_x3dh_secret(&dh_concat)
}

/// Derive a 32-byte shared secret from concatenated DH outputs using HKDF.
fn derive_x3dh_secret(dh_concat: &[u8]) -> [u8; 32] {
    // Use a fixed salt of 32 zero bytes (as specified by the X3DH spec).
    let salt = [0u8; 32];
    let hk = Hkdf::<Sha256>::new(Some(&salt), dh_concat);
    let mut shared_secret = [0u8; 32];
    hk.expand(b"X3DHSharedSecret", &mut shared_secret)
        .expect("32 bytes is a valid HKDF-SHA256 output length");
    shared_secret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x3dh_key_agreement_with_one_time_prekey() {
        // Alice's identity
        let alice_identity = IdentityKeyPair::generate();
        // Bob's keys
        let bob_identity = IdentityKeyPair::generate();
        let bob_signed = SignedPreKeyPair::generate();
        let bob_otpk = OneTimePreKeyPair::generate(1);

        // Bob publishes his bundle
        let bundle = PreKeyBundle {
            identity_key: bob_identity.public,
            signed_prekey: bob_signed.public,
            one_time_prekey: Some(bob_otpk.public),
        };

        // Alice initiates
        let alice_result = x3dh_initiate(&alice_identity, &bundle);

        // Bob responds
        let bob_secret = x3dh_respond(
            &bob_identity,
            &bob_signed,
            Some(&bob_otpk),
            &alice_identity.public,
            &alice_result.ephemeral_public,
        );

        assert_eq!(alice_result.shared_secret, bob_secret);
    }

    #[test]
    fn x3dh_key_agreement_without_one_time_prekey() {
        let alice_identity = IdentityKeyPair::generate();
        let bob_identity = IdentityKeyPair::generate();
        let bob_signed = SignedPreKeyPair::generate();

        let bundle = PreKeyBundle {
            identity_key: bob_identity.public,
            signed_prekey: bob_signed.public,
            one_time_prekey: None,
        };

        let alice_result = x3dh_initiate(&alice_identity, &bundle);

        let bob_secret = x3dh_respond(
            &bob_identity,
            &bob_signed,
            None,
            &alice_identity.public,
            &alice_result.ephemeral_public,
        );

        assert_eq!(alice_result.shared_secret, bob_secret);
    }

    #[test]
    fn x3dh_different_identities_produce_different_secrets() {
        let alice1 = IdentityKeyPair::generate();
        let alice2 = IdentityKeyPair::generate();
        let bob_identity = IdentityKeyPair::generate();
        let bob_signed = SignedPreKeyPair::generate();

        let bundle = PreKeyBundle {
            identity_key: bob_identity.public,
            signed_prekey: bob_signed.public,
            one_time_prekey: None,
        };

        let result1 = x3dh_initiate(&alice1, &bundle);
        let result2 = x3dh_initiate(&alice2, &bundle);

        // Different initiators should produce different shared secrets.
        assert_ne!(result1.shared_secret, result2.shared_secret);
    }
}
