use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::Claims, models::AppState, thread_comment::StatusResponse, validation::check_emails,
};

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/list", get(list_councils))
        .route("/create", post(create_council))
        .route("/update", post(update_council))
        .with_state(state)
}

#[derive(Serialize)]
struct CouncilBasic {
    name: String,
}

#[derive(Deserialize)]
struct CouncilFull {
    name: String,
    secretary_email: String,
    deputy_secretaries_email: Vec<String>,
}

#[derive(Deserialize)]
struct CouncilUpdateRequest {
    name: String,
    update: CouncilUpdate,
}

#[derive(Deserialize)]
enum CouncilUpdate {
    UpdateSeceratary(String),
    UpdateDeputySeceretaries(Vec<String>),
}

async fn list_councils(State(state): State<AppState>) -> Json<Vec<CouncilBasic>> {
    Json(
        sqlx::query_as!(CouncilBasic, "SELECT name FROM council",)
            .fetch_all(&state.connection)
            .await
            .unwrap(),
    )
}

async fn create_council(
    claims: Claims,
    State(state): State<AppState>,
    Json(council_data): Json<CouncilFull>,
) -> StatusResponse {
    match sqlx::query_scalar!("SELECT id FROM admin WHERE id = $1", claims.id)
        .fetch_one(&state.connection)
        .await
    {
        Ok(_) => {}
        Err(err) => return StatusResponse::UserError(err.to_string()),
    };

    match check_emails(
        &council_data.deputy_secretaries_email,
        state.connection.clone(),
    )
    .await
    {
        Ok(true) => {}
        Ok(false) => return StatusResponse::UserError("Deputy Seceretaries email is not valid".to_string()),
        Err(err) => return StatusResponse::UserError(err),
    };

    match sqlx::query!(
        "INSERT INTO council (name, secretary_email, deputy_secretaries_email) VALUES ($1, $2, $3)",
        &council_data.name,
        &council_data.secretary_email,
        &council_data.deputy_secretaries_email
    )
    .execute(&state.connection)
    .await
    {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    }
}

async fn update_council(
    State(state): State<AppState>,
    Json(update_req): Json<CouncilUpdateRequest>,
) -> StatusResponse {
    let name = update_req.name;

    let update_response = match update_req.update {
        CouncilUpdate::UpdateSeceratary(secretary) => sqlx::query!(
            "UPDATE council SET secretary_email = $1 WHERE name = $2",
            &secretary,
            name
        )
        .execute(&state.connection)
        .await
        .map_err(|e| e.to_string()),
        CouncilUpdate::UpdateDeputySeceretaries(deputies) => {
            match check_emails(&deputies, state.connection.clone()).await {
                Ok(true) => sqlx::query!(
                    "UPDATE council SET deputy_secretaries_email = $1 WHERE name = $2",
                    &deputies,
                    name
                )
                .execute(&state.connection)
                .await
                .map_err(|e| e.to_string()),
                Ok(false) => Err("One or more emails are not valid".to_string()),
                Err(err) => Err(err),
            }
        }
    };

    match update_response {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err),
    }
}
