use crate::db::schema::{identity_keys, one_time_prekeys, signed_prekeys};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ---------------------------------------------------------------------------
// Identity Keys
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = identity_keys)]
pub struct IdentityKey {
    pub id: i64,
    pub user_id: i64,
    /// Curve25519 public key (32 bytes, hex-encoded in JSON).
    #[schema(value_type = Vec<u8>)]
    pub public_key: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Upload or replace a user's identity public key.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadIdentityKey {
    /// Curve25519 public key (hex-encoded, 32 bytes / 64 hex chars).
    pub public_key: String,
}

// ---------------------------------------------------------------------------
// Signed Pre-Keys
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = signed_prekeys)]
pub struct SignedPrekey {
    pub id: i64,
    pub user_id: i64,
    /// Curve25519 public key (32 bytes).
    #[schema(value_type = Vec<u8>)]
    pub public_key: Vec<u8>,
    /// Ed25519 signature over the public key, produced by the identity key.
    #[schema(value_type = Vec<u8>)]
    pub signature: Vec<u8>,
    pub created_at: NaiveDateTime,
}

/// Upload a new signed pre-key.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadSignedPrekey {
    /// Curve25519 public key (hex-encoded).
    pub public_key: String,
    /// Ed25519 signature over the public key (hex-encoded).
    pub signature: String,
}

// ---------------------------------------------------------------------------
// One-Time Pre-Keys
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Queryable, Selectable, ToSchema)]
#[diesel(table_name = one_time_prekeys)]
pub struct OneTimePrekey {
    pub id: i64,
    pub user_id: i64,
    /// Curve25519 public key (32 bytes).
    #[schema(value_type = Vec<u8>)]
    pub public_key: Vec<u8>,
    pub created_at: NaiveDateTime,
}

/// Upload a batch of one-time pre-keys.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UploadOneTimePrekeys {
    /// List of Curve25519 public keys (each hex-encoded).
    pub prekeys: Vec<String>,
}

// ---------------------------------------------------------------------------
// Pre-Key Bundle (returned to initiators)
// ---------------------------------------------------------------------------

/// A bundle of public keys that a client needs to initiate an X3DH session
/// with another user.
#[derive(Debug, Serialize, ToSchema)]
pub struct PreKeyBundleResponse {
    /// The target user's identity public key (hex-encoded).
    pub identity_key: String,
    /// The target user's signed pre-key public key (hex-encoded).
    pub signed_prekey: String,
    /// Ed25519 signature over the signed pre-key (hex-encoded).
    pub signed_prekey_signature: String,
    /// An optional one-time pre-key (hex-encoded). Consumed after retrieval.
    pub one_time_prekey: Option<String>,
}
