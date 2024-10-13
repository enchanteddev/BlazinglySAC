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
        .route("/list", get(list_clubs))
        .route("/create", post(create_club))
        .route("/update", post(update_club))
        .with_state(state)
}

#[derive(Serialize)]
struct ClubBasic {
    name: String,
    description: String,
    council_name: String,
}

#[derive(Deserialize)]
struct ClubFull {
    name: String,
    email: String,
    description: String,
    council_name: String,
    club_head_emails: Vec<String>,
    phone: String,
}

#[derive(Deserialize)]
struct ClubUpdateRequest {
    name: String,
    update: ClubUpdate,
}

#[derive(Deserialize)]
enum ClubUpdate {
    UpdateHeads(Vec<String>),
    UpdateDescription(String),
    UpdatePhone(String),
    UpdateEmail(String),
}

async fn list_clubs(State(state): State<AppState>) -> Json<Vec<ClubBasic>> {
    Json(
        sqlx::query_as!(
            ClubBasic,
            "SELECT club.name, description, council.name as council_name FROM club INNER JOIN council ON club.council_id = council.id",
        )
        .fetch_all(&state.connection)
        .await
        .unwrap(),
    )
}

async fn create_club(
    claims: Claims,
    State(state): State<AppState>,
    Json(club_data): Json<ClubFull>,
) -> StatusResponse {
    match sqlx::query_scalar!("SELECT id FROM admin WHERE id = $1", claims.id)
        .fetch_one(&state.connection)
        .await
    {
        Ok(_) => {}
        Err(err) => return StatusResponse::UserError(err.to_string()),
    };

    let Ok(council_id) = sqlx::query_scalar!(
        "SELECT id FROM council WHERE name = $1",
        club_data.council_name
    )
    .fetch_one(&state.connection)
    .await
    else {
        return StatusResponse::UserError("Council not found".to_string());
    };

    match check_emails(&club_data.club_head_emails, state.connection.clone()).await {
        Ok(true) => {}
        Ok(false) => return StatusResponse::UserError("Club Heads email is not valid".to_string()),
        Err(err) => return StatusResponse::UserError(err),
    };

    match sqlx::query!(
        "INSERT INTO club (name, email, description, council_id, club_head_emails, phone) VALUES ($1, $2, $3, $4, $5, $6)",
        &club_data.name,
        &club_data.email,
        &club_data.description,
        council_id,
        &club_data.club_head_emails,
        &club_data.phone
    ).execute(&state.connection).await {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string())
    }
}

async fn update_club(
    State(state): State<AppState>,
    Json(update_req): Json<ClubUpdateRequest>,
) -> StatusResponse {
    let name = update_req.name;

    let update_response = match update_req.update {
        ClubUpdate::UpdateHeads(heads) => {
            match check_emails(&heads, state.connection.clone()).await {
                Ok(true) => sqlx::query!(
                    "UPDATE club SET club_head_emails = $1 WHERE name = $2",
                    &heads,
                    name
                )
                .execute(&state.connection)
                .await
                .map_err(|e| e.to_string()),
                Ok(false) => Err("One or more emails are not valid".to_string()),
                Err(err) => Err(err),
            }
        }

        ClubUpdate::UpdateDescription(desc) => sqlx::query!(
            "UPDATE club SET description = $1 WHERE name = $2",
            &desc,
            name
        )
        .execute(&state.connection)
        .await
        .map_err(|e| e.to_string()),

        ClubUpdate::UpdatePhone(phone) => {
            sqlx::query!("UPDATE club SET phone = $1 WHERE name = $2", &phone, name)
                .execute(&state.connection)
                .await
                .map_err(|e| e.to_string())
        }

        ClubUpdate::UpdateEmail(email) => {
            sqlx::query!("UPDATE club SET email = $1 WHERE name = $2", &email, name)
                .execute(&state.connection)
                .await
                .map_err(|e| e.to_string())
        }
    };

    match update_response {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err),
    }
}
