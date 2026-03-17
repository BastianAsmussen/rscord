use crate::api::auth_extractor::AuthUser;
use crate::api::errors::{ApiError, ErrorBody};
use crate::api::opaque::AppState;
use crate::db::models::relationships::{NewRelationship, Relationship, UpdateRelationship};
use crate::db::schema::relationships;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{post, put},
};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper, associations::HasTable};

type Pool = deadpool_diesel::postgres::Pool;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/relationships",
            post(create_relationship).get(get_relationships),
        )
        .route(
            "/api/relationships/{id}",
            put(update_relationship).delete(delete_relationship),
        )
}

/// Create a new relationship
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::Conflict`: If the provided user data violates database constraints.
/// - `ApiError::Internal`: If an error occurs during the database operation.
/// - `ApiError::Forbidden`: If a user tries to create a relationship with someone else as sender.
#[utoipa::path(
    post,
    path = "/api/relationships",
    request_body = NewRelationship,
    responses(
        (status = 201, description = "relationship created", body = Relationship),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 409, description = "Conflict", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "relationships"
)]
pub async fn create_relationship(
    auth: AuthUser,
    State(pool): State<Pool>,
    Json(payload): Json<NewRelationship>,
) -> Result<(StatusCode, Json<Relationship>), ApiError> {
    if auth.session.user_id != payload.sender_id {
        return Err(ApiError::Forbidden(
            "Can not create relationship with sender other then your own".into(),
        ));
    }

    let conn = pool.get().await?;

    let relationship: Relationship = conn
        .interact(|conn| {
            diesel::insert_into(relationships::dsl::relationships::table())
                .values(payload)
                .returning(Relationship::as_returning())
                .get_result(conn)
        })
        .await??;

    Ok((StatusCode::CREATED, Json(relationship)))
}

/// Retrieves a list of all relationships a user is in.
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::Internal`: If an error occurs during the database query execution.
#[utoipa::path(
    get,
    path = "/api/relationships",
    responses(
        (status = 200, description = "List of relationships", body = Vec<Relationship>),
        (status = 401, description = "Not authenticated", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "relationships"
)]
pub async fn get_relationships(
    auth: AuthUser,
    State(pool): State<Pool>,
) -> Result<Json<Vec<Relationship>>, ApiError> {
    let conn = pool.get().await?;

    let relationships: Vec<Relationship> = conn
        .interact(move |conn| {
            relationships::dsl::relationships
                .filter(relationships::dsl::sender_id.eq(&auth.session.user_id))
                .or_filter(relationships::dsl::receiver_id.eq(&auth.session.user_id))
                .load(conn)
        })
        .await??;

    Ok(Json(relationships))
}

/// Updates an existing relationship's data by their unique ID.
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::NotFound`: If no relationship exists with the given ID.
/// - `ApiError::Internal`: If an error occurs during the update operation in the database.
/// - `ApiError::Conflict`: If the updated data violates database constraints.
#[utoipa::path(
    put,
    path = "/api/relationships/{id}",
    params(("id" = i64, Path, description = "Relationship ID")),
    request_body = UpdateRelationship,
    responses(
        (status = 200, description = "Relationship updated", body = Relationship),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Relationship not found", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "relationships"
)]
pub async fn update_relationship(
    _auth: AuthUser,
    State(pool): State<Pool>,
    Path(relationship_id): Path<i64>,
    Json(payload): Json<UpdateRelationship>,
) -> Result<Json<Relationship>, ApiError> {
    let conn = pool.get().await?;

    let relationship: Relationship = conn
        .interact(move |conn| {
            diesel::update(
                relationships::dsl::relationships
                    .filter(relationships::dsl::id.eq(relationship_id)),
            )
            .set(&payload)
            .returning(Relationship::as_returning())
            .get_result(conn)
        })
        .await?
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                ApiError::NotFound(format!("relationship {relationship_id} not found"))
            }
            other => other.into(),
        })?;

    Ok(Json(relationship))
}

/// Deletes a relationship by their unique ID.
///
/// # Errors
///
/// This function may return the following errors:
/// - `ApiError::NotFound`: If the relationship with the given ID is not found.
/// - `ApiError::Internal`: If an error occurs during the database operation.
#[utoipa::path(
    delete,
    path = "/api/relationships/{id}",
    params(("id" = i64, Path, description = "Relationship ID")),
    responses(
        (status = 204, description = "Relationship deleted"),
        (status = 401, description = "Not authenticated", body = ErrorBody),
        (status = 404, description = "Relationship not found", body = ErrorBody),
    ),
    security(("session_token" = [])),
    tag = "relationships"
)]
pub async fn delete_relationship(
    _auth: AuthUser,
    State(pool): State<Pool>,
    Path(relationship_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let conn = pool.get().await?;

    let rows_deleted: usize = conn
        .interact(move |conn| {
            diesel::delete(
                relationships::dsl::relationships
                    .filter(relationships::dsl::id.eq(relationship_id)),
            )
            .execute(conn)
        })
        .await??;

    if rows_deleted == 0 {
        return Err(ApiError::NotFound(format!(
            "relationship {relationship_id} not found"
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}
