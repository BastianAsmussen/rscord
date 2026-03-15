//! Double Ratchet Algorithm implementation.
//!
//! Follows the Signal specification:
//! <https://signal.org/docs/specifications/doubleratchet/>
//!
//! The Double Ratchet combines a symmetric-key ratchet (KDF chain) with a
//! Diffie-Hellman ratchet to provide forward secrecy and break-in recovery.
//!
//! # Wire format
//!
//! Each encrypted message is a [`RatchetMessage`] containing:
//! - The sender's current DH ratchet public key
//! - Counters for the current and previous sending chains
//! - Ciphertext encrypted with AES-256-GCM
//! - The AES-GCM nonce

use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use rand_core_06::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use x25519_dalek::{PublicKey, StaticSecret};

use super::kdf::{kdf_ck, kdf_rk};

/// Maximum number of skipped message keys we keep before discarding.
const MAX_SKIP: u32 = 256;

/// The ratchet state held by one party.
///
/// Both the sender and receiver maintain one of these, updating it as messages
/// are sent and received.
pub struct DoubleRatchet {
    /// Our current DH ratchet key pair.
    dh_self_secret: StaticSecret,
    dh_self_public: PublicKey,
    /// The remote party's current DH ratchet public key.
    dh_remote: Option<PublicKey>,

    /// Root key (32 bytes) - ratcheted on every DH ratchet step.
    root_key: [u8; 32],
    /// Sending chain key.
    chain_key_send: Option<[u8; 32]>,
    /// Receiving chain key.
    chain_key_recv: Option<[u8; 32]>,

    /// Number of messages sent in the current sending chain.
    send_count: u32,
    /// Number of messages received in the current receiving chain.
    recv_count: u32,
    /// Number of messages in the previous sending chain (for header).
    prev_send_count: u32,

    /// Skipped message keys, indexed by `(ratchet_public_key_bytes, chain_index)`.
    skipped_keys: HashMap<([u8; 32], u32), [u8; 32]>,
}

/// A message produced by the Double Ratchet, ready for transmission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatchetMessage {
    /// The sender's current ratchet public key (32 bytes).
    pub ratchet_pub: [u8; 32],
    /// Number of messages in the previous sending chain.
    pub prev_chain_len: u32,
    /// Message number in the current sending chain.
    pub msg_num: u32,
    /// AES-256-GCM ciphertext.
    pub ciphertext: Vec<u8>,
    /// AES-256-GCM nonce (12 bytes).
    pub nonce: [u8; 12],
}

impl DoubleRatchet {
    /// Initialise the ratchet as the **initiator** (Alice).
    ///
    /// `shared_secret` is the output of X3DH. `their_ratchet_key` is Bob's
    /// signed pre-key public key (which doubles as his initial ratchet key).
    #[must_use]
    pub fn init_alice(shared_secret: [u8; 32], their_ratchet_key: PublicKey) -> Self {
        let dh_self_secret = StaticSecret::random_from_rng(OsRng);
        let dh_self_public = PublicKey::from(&dh_self_secret);

        // Perform initial DH ratchet step
        let dh_out = dh_self_secret.diffie_hellman(&their_ratchet_key);
        let (root_key, chain_key_send) = kdf_rk(&shared_secret, dh_out.as_bytes());

        Self {
            dh_self_secret,
            dh_self_public,
            dh_remote: Some(their_ratchet_key),
            root_key,
            chain_key_send: Some(chain_key_send),
            chain_key_recv: None,
            send_count: 0,
            recv_count: 0,
            prev_send_count: 0,
            skipped_keys: HashMap::new(),
        }
    }

    /// Initialise the ratchet as the **responder** (Bob).
    ///
    /// `shared_secret` is the output of X3DH. `our_signed_prekey` is the
    /// secret half of the signed pre-key used during X3DH (it doubles as
    /// Bob's initial ratchet key).
    #[must_use]
    pub fn init_bob(shared_secret: [u8; 32], our_signed_prekey: StaticSecret) -> Self {
        let dh_self_public = PublicKey::from(&our_signed_prekey);

        Self {
            dh_self_secret: our_signed_prekey,
            dh_self_public,
            dh_remote: None,
            root_key: shared_secret,
            chain_key_send: None,
            chain_key_recv: None,
            send_count: 0,
            recv_count: 0,
            prev_send_count: 0,
            skipped_keys: HashMap::new(),
        }
    }

    /// Encrypt a plaintext message and advance the sending ratchet.
    ///
    /// # Panics
    ///
    /// Panics if the sending chain has not been initialized (should not happen
    /// in normal operation after `init_alice` or after receiving the first
    /// message as Bob).
    pub fn encrypt(&mut self, plaintext: &[u8]) -> RatchetMessage {
        let ck = self
            .chain_key_send
            .expect("sending chain not initialized; did you receive a message first as Bob?");

        let (next_ck, message_key) = kdf_ck(&ck);
        self.chain_key_send = Some(next_ck);

        let (ciphertext, nonce) = aes_gcm_encrypt(&message_key, plaintext);

        let msg = RatchetMessage {
            ratchet_pub: *self.dh_self_public.as_bytes(),
            prev_chain_len: self.prev_send_count,
            msg_num: self.send_count,
            ciphertext,
            nonce,
        };

        self.send_count += 1;
        msg
    }

    /// Decrypt a received [`RatchetMessage`] and advance the receiving ratchet.
    ///
    /// Handles out-of-order messages by caching skipped message keys (up to
    /// [`MAX_SKIP`]).
    ///
    /// # Errors
    ///
    /// Returns an error string if decryption fails or too many messages were
    /// skipped.
    pub fn decrypt(&mut self, msg: &RatchetMessage) -> Result<Vec<u8>, String> {
        // 1. Try skipped keys first.
        let key = (msg.ratchet_pub, msg.msg_num);
        if let Some(mk) = self.skipped_keys.remove(&key) {
            return aes_gcm_decrypt(&mk, &msg.ciphertext, &msg.nonce);
        }

        // 2. If the ratchet key changed, perform a DH ratchet step.
        let their_pub = PublicKey::from(msg.ratchet_pub);
        if self.dh_remote.as_ref().map(PublicKey::as_bytes) != Some(&msg.ratchet_pub) {
            self.skip_message_keys(msg.prev_chain_len)?;
            self.dh_ratchet_step(their_pub);
        }

        // 3. Skip any messages in the current receiving chain that we haven't
        //    seen yet (out-of-order delivery).
        self.skip_message_keys(msg.msg_num)?;

        // 4. Derive the message key from the current receiving chain.
        let ck = self
            .chain_key_recv
            .ok_or("receiving chain not initialized")?;
        let (next_ck, message_key) = kdf_ck(&ck);
        self.chain_key_recv = Some(next_ck);
        self.recv_count += 1;

        aes_gcm_decrypt(&message_key, &msg.ciphertext, &msg.nonce)
    }

    /// Perform a DH ratchet step: generate a new DH key pair, advance the
    /// root key, and reset the sending/receiving chains.
    fn dh_ratchet_step(&mut self, their_new_public: PublicKey) {
        self.prev_send_count = self.send_count;
        self.send_count = 0;
        self.recv_count = 0;
        self.dh_remote = Some(their_new_public);

        // Receiving chain
        let dh_recv = self.dh_self_secret.diffie_hellman(&their_new_public);
        let (root_key, chain_key_recv) = kdf_rk(&self.root_key, dh_recv.as_bytes());
        self.root_key = root_key;
        self.chain_key_recv = Some(chain_key_recv);

        // New DH key pair for sending
        self.dh_self_secret = StaticSecret::random_from_rng(OsRng);
        self.dh_self_public = PublicKey::from(&self.dh_self_secret);

        // Sending chain
        let dh_send = self.dh_self_secret.diffie_hellman(&their_new_public);
        let (root_key, chain_key_send) = kdf_rk(&self.root_key, dh_send.as_bytes());
        self.root_key = root_key;
        self.chain_key_send = Some(chain_key_send);
    }

    /// Cache message keys for skipped messages in the current receiving chain.
    fn skip_message_keys(&mut self, until: u32) -> Result<(), String> {
        if self.recv_count + MAX_SKIP < until {
            return Err("too many skipped messages".to_owned());
        }
        if let Some(mut ck) = self.chain_key_recv {
            while self.recv_count < until {
                let (next_ck, mk) = kdf_ck(&ck);
                let ratchet_pub = self.dh_remote.map_or([0u8; 32], |pk| *pk.as_bytes());
                self.skipped_keys.insert((ratchet_pub, self.recv_count), mk);
                ck = next_ck;
                self.chain_key_recv = Some(ck);
                self.recv_count += 1;
            }
        }
        Ok(())
    }
}

/// Encrypt plaintext with AES-256-GCM.  Returns `(ciphertext, nonce)`.
fn aes_gcm_encrypt(key: &[u8; 32], plaintext: &[u8]) -> (Vec<u8>, [u8; 12]) {
    use rand_core_06::RngCore;

    let cipher = Aes256Gcm::new_from_slice(key).expect("32-byte key is valid for AES-256-GCM");
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .expect("AES-GCM encryption should not fail");

    (ciphertext, nonce_bytes)
}

/// Decrypt ciphertext with AES-256-GCM.
fn aes_gcm_decrypt(
    key: &[u8; 32],
    ciphertext: &[u8],
    nonce_bytes: &[u8; 12],
) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).expect("32-byte key is valid for AES-256-GCM");
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "AES-GCM decryption failed (bad key or tampered ciphertext)".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Full round-trip: Alice sends to Bob, Bob replies to Alice.
    #[test]
    fn basic_round_trip() {
        let shared_secret = [0x42u8; 32];
        let bob_spk_secret = StaticSecret::random_from_rng(OsRng);
        let bob_spk_public = PublicKey::from(&bob_spk_secret);

        let mut alice = DoubleRatchet::init_alice(shared_secret, bob_spk_public);
        let mut bob = DoubleRatchet::init_bob(shared_secret, bob_spk_secret);

        // Alice -> Bob
        let msg1 = alice.encrypt(b"Hello Bob!");
        let pt1 = bob.decrypt(&msg1).expect("Bob should decrypt Alice's msg");
        assert_eq!(pt1, b"Hello Bob!");

        // Bob -> Alice
        let msg2 = bob.encrypt(b"Hi Alice!");
        let pt2 = alice
            .decrypt(&msg2)
            .expect("Alice should decrypt Bob's msg");
        assert_eq!(pt2, b"Hi Alice!");
    }

    /// Multiple messages in the same direction.
    #[test]
    fn multiple_messages_same_direction() {
        let shared_secret = [0x01u8; 32];
        let bob_spk_secret = StaticSecret::random_from_rng(OsRng);
        let bob_spk_public = PublicKey::from(&bob_spk_secret);

        let mut alice = DoubleRatchet::init_alice(shared_secret, bob_spk_public);
        let mut bob = DoubleRatchet::init_bob(shared_secret, bob_spk_secret);

        let msgs: Vec<RatchetMessage> = (0..5)
            .map(|i| alice.encrypt(format!("msg {i}").as_bytes()))
            .collect();

        for (i, msg) in msgs.iter().enumerate() {
            let pt = bob.decrypt(msg).expect("decryption should succeed");
            assert_eq!(pt, format!("msg {i}").as_bytes());
        }
    }

    /// Out-of-order delivery within a single chain.
    #[test]
    fn out_of_order_messages() {
        let shared_secret = [0x02u8; 32];
        let bob_spk_secret = StaticSecret::random_from_rng(OsRng);
        let bob_spk_public = PublicKey::from(&bob_spk_secret);

        let mut alice = DoubleRatchet::init_alice(shared_secret, bob_spk_public);
        let mut bob = DoubleRatchet::init_bob(shared_secret, bob_spk_secret);

        let msg0 = alice.encrypt(b"first");
        let msg1 = alice.encrypt(b"second");
        let msg2 = alice.encrypt(b"third");

        // Deliver out of order: 2, 0, 1
        let pt2 = bob.decrypt(&msg2).expect("decrypt msg2");
        assert_eq!(pt2, b"third");

        let pt0 = bob.decrypt(&msg0).expect("decrypt msg0");
        assert_eq!(pt0, b"first");

        let pt1 = bob.decrypt(&msg1).expect("decrypt msg1");
        assert_eq!(pt1, b"second");
    }

    /// Alternating conversation simulates a realistic chat.
    #[test]
    fn alternating_conversation() {
        let shared_secret = [0x03u8; 32];
        let bob_spk_secret = StaticSecret::random_from_rng(OsRng);
        let bob_spk_public = PublicKey::from(&bob_spk_secret);

        let mut alice = DoubleRatchet::init_alice(shared_secret, bob_spk_public);
        let mut bob = DoubleRatchet::init_bob(shared_secret, bob_spk_secret);

        for round in 0..10 {
            let a_msg = alice.encrypt(format!("Alice round {round}").as_bytes());
            let a_pt = bob.decrypt(&a_msg).expect("Bob decrypts Alice");
            assert_eq!(a_pt, format!("Alice round {round}").as_bytes());

            let b_msg = bob.encrypt(format!("Bob round {round}").as_bytes());
            let b_pt = alice.decrypt(&b_msg).expect("Alice decrypts Bob");
            assert_eq!(b_pt, format!("Bob round {round}").as_bytes());
        }
    }

    /// Tampered ciphertext must fail decryption.
    #[test]
    fn tampered_ciphertext_fails() {
        let shared_secret = [0x04u8; 32];
        let bob_spk_secret = StaticSecret::random_from_rng(OsRng);
        let bob_spk_public = PublicKey::from(&bob_spk_secret);

        let mut alice = DoubleRatchet::init_alice(shared_secret, bob_spk_public);
        let mut bob = DoubleRatchet::init_bob(shared_secret, bob_spk_secret);

        let mut msg = alice.encrypt(b"secret data");
        // Flip a byte in the ciphertext.
        if let Some(byte) = msg.ciphertext.first_mut() {
            *byte ^= 0xFF;
        }

        assert!(bob.decrypt(&msg).is_err());
    }

    /// Empty plaintext is a valid message.
    #[test]
    fn empty_plaintext() {
        let shared_secret = [0x05u8; 32];
        let bob_spk_secret = StaticSecret::random_from_rng(OsRng);
        let bob_spk_public = PublicKey::from(&bob_spk_secret);

        let mut alice = DoubleRatchet::init_alice(shared_secret, bob_spk_public);
        let mut bob = DoubleRatchet::init_bob(shared_secret, bob_spk_secret);

        let msg = alice.encrypt(b"");
        let pt = bob.decrypt(&msg).expect("decrypt empty");
        assert!(pt.is_empty());
    }
}
