use crate::{
    db::model::Invitation,
    error::AppResult,
    extractor::{AdminGuard, ValidatedJson, ValidatedQuery},
    AppState,
};
use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

// ========================// Invitation Router //======================== //

/// Create user router
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route(
        "/invitation",
        get(list_invitations)
            .post(create_invitation)
            .delete(delete_invitations),
    )
}

// ========================// Create Invitation //======================== //

#[derive(Deserialize, Validate)]
pub struct CreateInvitationRequest {
    #[validate(range(min = 4, message = "must be greater than 3"))]
    pub length: usize,
    #[validate(range(min = 1, message = "must be greater than 0"))]
    pub days: u64,
}

#[derive(Serialize)]
pub struct CreateInvitationResponse {
    pub invitation: Invitation,
}

async fn create_invitation(
    State(state): State<Arc<AppState>>,
    AdminGuard(_): AdminGuard,
    ValidatedJson(req): ValidatedJson<CreateInvitationRequest>,
) -> AppResult<Json<CreateInvitationResponse>> {
    let invitation = state.db.create_invitation(&req.into()).await?;

    Ok(Json(CreateInvitationResponse { invitation }))
}

// ========================// Delete Invitations //======================== //

#[derive(Deserialize, Validate)]
pub struct DeleteInvitationsRequest {
    #[validate(length(min = 1, message = "must be greater than 1"))]
    pub codes: Vec<String>,
}

async fn delete_invitations(
    State(state): State<Arc<AppState>>,
    AdminGuard(_): AdminGuard,
    ValidatedJson(req): ValidatedJson<DeleteInvitationsRequest>,
) -> AppResult<()> {
    state.db.delete_invitations(&req.codes).await?;

    Ok(())
}

// ========================// List Invitations //======================== //

#[derive(Deserialize, Validate)]
pub struct ListInvitationsRequest {
    #[validate(range(min = 1, message = "must be greater than 1"))]
    pub page_id: Option<i64>,
    #[validate(range(min = 5, max = 50, message = "must be between 5 and 50"))]
    pub page_size: Option<i64>,
}

#[derive(Serialize)]
pub struct ListInvitationsResponse {
    pub total: i64,
    pub invitations: Vec<Invitation>,
}

async fn list_invitations(
    State(state): State<Arc<AppState>>,
    AdminGuard(_): AdminGuard,
    ValidatedQuery(req): ValidatedQuery<ListInvitationsRequest>,
) -> AppResult<Json<ListInvitationsResponse>> {
    let (total, invitations) = state.db.list_invitations(&req.into()).await?;

    Ok(Json(ListInvitationsResponse { total, invitations }))
}
