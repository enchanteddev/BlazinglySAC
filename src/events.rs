use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::{auth::Claims, models::AppState, thread_comment::StatusResponse};

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/view", get(events))
        .route("/create", post(create_event))
        .with_state(state)
}

#[derive(FromRow, Serialize)]
struct Event {
    pub id: i32,
    pub club_name: String,
    pub title: String,
    pub description: String,
    pub starts_at: DateTime<Utc>,
    pub venue: String,
}

#[derive(Deserialize)]
struct EventRequest {
    title: String,
    description: String,
    club_id: i32,
    starts_at: DateTime<Utc>,
    venue: String,
}

async fn events(State(state): State<AppState>) -> Json<Vec<Event>> {
    Json(
        sqlx::query_as::<_, Event>(
            "SELECT event.id, club.name as club_name, title, event.description, starts_at, venue FROM event INNER JOIN club ON club.id = event.club_id ORDER BY event.id DESC",
        )
        .fetch_all(&state.connection)
        .await
        .unwrap(),
    )
}

async fn create_event(
    claims: Claims,
    State(state): State<AppState>,
    Json(request): Json<EventRequest>,
) -> impl IntoResponse {
    let club_id = request.club_id;
    let user_id = claims.id;
    let Ok(privilege_level) = sqlx::query_scalar!(
        // For some reason the `!` is needed here to force non-nullablity
        "
            SELECT privilege_level AS \"privilege_level!\" FROM membership
            WHERE user_id = $1 AND club_id = $2
            LIMIT 1
        ",
        user_id,
        club_id
    )
    .fetch_one(&state.connection)
    .await
    else {
        return StatusResponse::ServerError;
    };

    if privilege_level <= 1 {
        return StatusResponse::UserError(
            "You are not allowed to create announcements in this club".to_string(),
        );
    }

    match sqlx::query_scalar!(
        "INSERT INTO event (title, description, user_id, club_id, starts_at, venue) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        &request.title,
        &request.description,
        user_id,
        club_id,
        &request.starts_at,
        &request.venue,
    )
    .fetch_one(&state.connection)
    .await
    {
        Ok(id) => StatusResponse::SuccessWithData(id.to_string()),
        Err(err) => StatusResponse::UserError(err.to_string()),
    }
}
