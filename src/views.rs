use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::prelude::FromRow;

use crate::models::AppState;

#[derive(FromRow, Serialize)]
pub struct PublicAnnouncement {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub created_at: String,
}


pub async fn announcements(State(state): State<AppState>) -> Json<Vec<PublicAnnouncement>> {
    Json(
        sqlx::query_as::<_, PublicAnnouncement>(
            "SELECT (id, title, content, created_at) FROM announcement",
        )
        .fetch_all(&state.connection)
        .await
        .unwrap(),
    )
}
