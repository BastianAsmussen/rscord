use crate::api::auth_extractor::AuthUser;
use crate::api::errors::ApiError;
use crate::api::opaque::AppState;
use crate::db::models::keys::{
    IdentityKey, OneTimePrekey, PreKeyBundleResponse, SignedPrekey, UploadIdentityKey,
    UploadOneTimePrekeys, UploadSignedPrekey,
};
use crate::db::schema::{identity_keys, one_time_prekeys, signed_prekeys};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use diesel::prelude::*;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/keys/identity", post(upload_identity_key))
        .route("/api/keys/signed-prekey", post(upload_signed_prekey))
        .route("/api/keys/prekeys", post(upload_one_time_prekeys))
        .route("/api/keys/bundle/{user_id}", get(get_prekey_bundle))
}

/// Upload or replace the caller's identity public key.
///
/// This key is the long-lived Curve25519 public key used in X3DH and the
/// Double Ratchet protocol. Each user has exactly one identity key.
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user session is missing or invalid.
/// - `ApiError::UnprocessableEntity`: If the public key is malformed.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    post,
    path = "/api/keys/identity",
    request_body = UploadIdentityKey,
    responses(
        (status = 201, description = "Identity key stored", body = IdentityKey),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn upload_identity_key(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UploadIdentityKey>,
) -> Result<(StatusCode, Json<IdentityKey>), ApiError> {
    let public_key_bytes = hex::decode(&payload.public_key)
        .map_err(|_| ApiError::UnprocessableEntity("Invalid hex in public_key".into()))?;

    if public_key_bytes.len() != 32 {
        return Err(ApiError::UnprocessableEntity(
            "public_key must be exactly 32 bytes (64 hex chars)".into(),
        ));
    }

    let conn = state.pool.get().await?;
    let user_id = auth.session.user_id;

    let key = conn
        .interact(move |conn| {
            diesel::insert_into(identity_keys::table)
                .values((
                    identity_keys::user_id.eq(user_id),
                    identity_keys::public_key.eq(&public_key_bytes),
                ))
                .on_conflict(identity_keys::user_id)
                .do_update()
                .set((
                    identity_keys::public_key.eq(&public_key_bytes),
                    identity_keys::updated_at.eq(diesel::dsl::now),
                ))
                .returning(IdentityKey::as_returning())
                .get_result::<IdentityKey>(conn)
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(key)))
}

/// Upload a new signed pre-key.
///
/// The signed pre-key is a medium-term Curve25519 public key signed by the
/// user's Ed25519 identity signing key. Clients rotate this periodically.
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user session is missing or invalid.
/// - `ApiError::UnprocessableEntity`: If the key or signature is malformed.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    post,
    path = "/api/keys/signed-prekey",
    request_body = UploadSignedPrekey,
    responses(
        (status = 201, description = "Signed pre-key stored", body = SignedPrekey),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn upload_signed_prekey(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UploadSignedPrekey>,
) -> Result<(StatusCode, Json<SignedPrekey>), ApiError> {
    let public_key_bytes = hex::decode(&payload.public_key)
        .map_err(|_| ApiError::UnprocessableEntity("Invalid hex in public_key".into()))?;
    let signature_bytes = hex::decode(&payload.signature)
        .map_err(|_| ApiError::UnprocessableEntity("Invalid hex in signature".into()))?;

    if public_key_bytes.len() != 32 {
        return Err(ApiError::UnprocessableEntity(
            "public_key must be exactly 32 bytes".into(),
        ));
    }

    let conn = state.pool.get().await?;
    let user_id = auth.session.user_id;

    let key = conn
        .interact(move |conn| {
            diesel::insert_into(signed_prekeys::table)
                .values((
                    signed_prekeys::user_id.eq(user_id),
                    signed_prekeys::public_key.eq(&public_key_bytes),
                    signed_prekeys::signature.eq(&signature_bytes),
                ))
                .returning(SignedPrekey::as_returning())
                .get_result::<SignedPrekey>(conn)
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(key)))
}

/// Upload a batch of one-time pre-keys.
///
/// One-time pre-keys are single-use Curve25519 public keys. Each is consumed
/// (deleted) when another user fetches a pre-key bundle. Clients should
/// upload new batches as their supply runs low.
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the user session is missing or invalid.
/// - `ApiError::UnprocessableEntity`: If any key is malformed.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    post,
    path = "/api/keys/prekeys",
    request_body = UploadOneTimePrekeys,
    responses(
        (status = 201, description = "One-time pre-keys stored"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn upload_one_time_prekeys(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UploadOneTimePrekeys>,
) -> Result<StatusCode, ApiError> {
    let mut decoded_keys = Vec::with_capacity(payload.prekeys.len());
    for (i, hex_key) in payload.prekeys.iter().enumerate() {
        let bytes = hex::decode(hex_key)
            .map_err(|_| ApiError::UnprocessableEntity(format!("Invalid hex in prekeys[{i}]")))?;
        if bytes.len() != 32 {
            return Err(ApiError::UnprocessableEntity(format!(
                "prekeys[{i}] must be exactly 32 bytes"
            )));
        }
        decoded_keys.push(bytes);
    }

    let conn = state.pool.get().await?;
    let user_id = auth.session.user_id;

    conn.interact(move |conn| {
        let values: Vec<_> = decoded_keys
            .iter()
            .map(|pk| {
                (
                    one_time_prekeys::user_id.eq(user_id),
                    one_time_prekeys::public_key.eq(pk),
                )
            })
            .collect();

        diesel::insert_into(one_time_prekeys::table)
            .values(&values)
            .execute(conn)
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::CREATED)
}

/// Fetch the pre-key bundle for a user (for initiating an X3DH session).
///
/// Returns the target user's identity key, latest signed pre-key, and an
/// optional one-time pre-key. The one-time pre-key is consumed (deleted) to
/// prevent reuse.
///
/// # Errors
///
/// - `ApiError::Unauthorized`: If the caller session is missing or invalid.
/// - `ApiError::NotFound`: If the target user has no identity or signed pre-key.
/// - `ApiError::Internal`: If the database operation fails.
#[utoipa::path(
    get,
    path = "/api/keys/bundle/{user_id}",
    responses(
        (status = 200, description = "Pre-key bundle", body = PreKeyBundleResponse),
        (status = 404, description = "User has no keys uploaded")
    ),
    params(("user_id" = i64, Path, description = "Target user ID"))
)]
pub async fn get_prekey_bundle(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(target_user_id): Path<i64>,
) -> Result<Json<PreKeyBundleResponse>, ApiError> {
    let conn = state.pool.get().await?;

    let bundle = conn
        .interact(move |conn| -> Result<PreKeyBundleResponse, ApiError> {
            // Identity key (required).
            let ik: IdentityKey = identity_keys::table
                .filter(identity_keys::user_id.eq(target_user_id))
                .select(IdentityKey::as_select())
                .first(conn)
                .map_err(|_| ApiError::NotFound("User has no identity key uploaded".into()))?;

            // Latest signed pre-key (required).
            let spk: SignedPrekey = signed_prekeys::table
                .filter(signed_prekeys::user_id.eq(target_user_id))
                .order(signed_prekeys::created_at.desc())
                .select(SignedPrekey::as_select())
                .first(conn)
                .map_err(|_| ApiError::NotFound("User has no signed pre-key uploaded".into()))?;

            // One-time pre-key (optional, consumed on fetch).
            let otpk: Option<OneTimePrekey> = one_time_prekeys::table
                .filter(one_time_prekeys::user_id.eq(target_user_id))
                .order(one_time_prekeys::created_at.asc())
                .select(OneTimePrekey::as_select())
                .first(conn)
                .optional()
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            // Consume the one-time pre-key.
            if let Some(ref opk) = otpk {
                diesel::delete(one_time_prekeys::table.find(opk.id))
                    .execute(conn)
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
            }

            Ok(PreKeyBundleResponse {
                identity_key: hex::encode(&ik.public_key),
                signed_prekey: hex::encode(&spk.public_key),
                signed_prekey_signature: hex::encode(&spk.signature),
                one_time_prekey: otpk.map(|k| hex::encode(&k.public_key)),
            })
        })
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(bundle))
}
