//! Key derivation functions used by the Double Ratchet and X3DH protocols.
//!
//! Follows the Signal specification:
//! <https://signal.org/docs/specifications/doubleratchet/#recommended-cryptographic-algorithms>

use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// HKDF-based root key derivation.
///
/// Takes the current root key and DH output, returns a new
/// (`root_key`, `chain_key`) pair.
/// This implements the `KDF_RK` function from the Signal Double Ratchet specification.
///
/// # Panics
///
/// Panics if HKDF expansion fails (should never happen with valid inputs).
#[must_use]
pub fn kdf_rk(root_key: &[u8; 32], dh_out: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let hk = Hkdf::<Sha256>::new(Some(root_key), dh_out);
    let mut okm = [0u8; 64];
    hk.expand(b"DoubleRatchetRK", &mut okm)
        .expect("64 bytes is a valid HKDF-SHA256 output length");

    let mut new_root_key = [0u8; 32];
    let mut chain_key = [0u8; 32];
    new_root_key.copy_from_slice(&okm[..32]);
    chain_key.copy_from_slice(&okm[32..]);

    (new_root_key, chain_key)
}

/// HMAC-based chain key derivation.
///
/// Takes a chain key and returns the next (`chain_key`, `message_key`) pair.
/// This implements the `KDF_CK` function from the Signal Double Ratchet specification.
///
/// # Panics
///
/// Panics if HMAC initialization fails (should never happen with valid inputs).
#[must_use]
pub fn kdf_ck(chain_key: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    // Message key: HMAC(ck, 0x01)
    let mut mac = HmacSha256::new_from_slice(chain_key).expect("HMAC can take a key of any size");
    mac.update(&[0x01]);
    let message_key: [u8; 32] = mac.finalize().into_bytes().into();

    // Next chain key: HMAC(ck, 0x02)
    let mut mac = HmacSha256::new_from_slice(chain_key).expect("HMAC can take a key of any size");
    mac.update(&[0x02]);
    let next_chain_key: [u8; 32] = mac.finalize().into_bytes().into();

    (next_chain_key, message_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kdf_rk_produces_distinct_keys() {
        let root_key = [0xAA; 32];
        let dh_out = [0xBB; 32];

        let (new_rk, ck) = kdf_rk(&root_key, &dh_out);

        // The derived keys must differ from each other and from input.
        assert_ne!(new_rk, ck);
        assert_ne!(new_rk, root_key);
        assert_ne!(ck, root_key);
    }

    #[test]
    fn kdf_rk_is_deterministic() {
        let root_key = [0x42; 32];
        let dh_out = [0x13; 32];

        let (rk1, ck1) = kdf_rk(&root_key, &dh_out);
        let (rk2, ck2) = kdf_rk(&root_key, &dh_out);

        assert_eq!(rk1, rk2);
        assert_eq!(ck1, ck2);
    }

    #[test]
    fn kdf_ck_produces_distinct_keys() {
        let chain_key = [0xCC; 32];
        let (next_ck, mk) = kdf_ck(&chain_key);

        assert_ne!(next_ck, mk);
        assert_ne!(next_ck, chain_key);
        assert_ne!(mk, chain_key);
    }

    #[test]
    fn kdf_ck_is_deterministic() {
        let chain_key = [0xDD; 32];

        let (ck1, mk1) = kdf_ck(&chain_key);
        let (ck2, mk2) = kdf_ck(&chain_key);

        assert_eq!(ck1, ck2);
        assert_eq!(mk1, mk2);
    }

    #[test]
    fn kdf_ck_chain_advances() {
        let ck0 = [0xEE; 32];
        let (ck1, mk1) = kdf_ck(&ck0);
        let (ck2, mk2) = kdf_ck(&ck1);

        // Each step produces different keys.
        assert_ne!(mk1, mk2);
        assert_ne!(ck1, ck2);
    }
}
