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

#[derive(FromRow, Serialize)]
pub struct PublicCouncil {
    pub id: i32,
    pub name: String,
    pub secretary_name: String,
    pub deputy_secretaries_name: Vec<String>,
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

pub async fn councils(State(state): State<AppState>) -> Json<Vec<PublicCouncil>> {
    Json(
        sqlx::query_as::<_, PublicCouncil>(
            "SELECT (id, name, secretary_name, deputy_secretaries_name) FROM council",
        )
        .fetch_all(&state.connection)
        .await
        .unwrap(),
    )
}
