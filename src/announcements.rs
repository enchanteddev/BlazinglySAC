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
        .route("/public", get(announcements))
        .route("/create", post(create_announcement))
        .with_state(state)
}

#[derive(FromRow, Serialize)]
struct PublicAnnouncement {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct AnnouncementRequest {
    title: String,
    content: String,
    club_id: i32,
}

async fn announcements(State(state): State<AppState>) -> Json<Vec<PublicAnnouncement>> {
    Json(
        sqlx::query_as::<_, PublicAnnouncement>(
            "SELECT id, title, content, created_at FROM announcement",
        )
        .fetch_all(&state.connection)
        .await
        .unwrap(),
    )
}

async fn create_announcement(
    claims: Claims,
    State(state): State<AppState>,
    Json(request): Json<AnnouncementRequest>,
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

    match sqlx::query!(
        "INSERT INTO announcement (title, content, club_id) VALUES ($1, $2, $3) RETURNING id",
        &request.title,
        &request.content,
        club_id
    )
    .fetch_one(&state.connection)
    .await
    {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    }
}
