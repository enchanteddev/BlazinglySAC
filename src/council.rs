use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{auth::Claims, models::AppState, thread_comment::StatusResponse};

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
    secretary_name: String,
    deputy_secretaries_name: Vec<String>,
}

#[derive(Deserialize)]
struct  CouncilUpdateRequest {
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

    match sqlx::query!(
        "INSERT INTO council (name, secretary_name, deputy_secretaries_name) VALUES ($1, $2, $3)",
        &council_data.name,
        &council_data.secretary_name,
        &council_data.deputy_secretaries_name
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
        CouncilUpdate::UpdateSeceratary(secretary) => {
            sqlx::query!(
                "UPDATE council SET secretary_name = $1 WHERE name = $2",
                &secretary,
                name
            )
            .execute(&state.connection)
            .await
        }
        CouncilUpdate::UpdateDeputySeceretaries(deputies) => {
            sqlx::query!(
                "UPDATE council SET deputy_secretaries_name = $1 WHERE name = $2",
                &deputies,
                name
            )
            .execute(&state.connection)
            .await
        }
    };

    match update_response {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    }
}
