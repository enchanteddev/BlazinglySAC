use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::models::AppState;

#[derive(Deserialize)]
pub struct CommentRequest {
    pub thread_id: i32,
}

#[derive(FromRow, Serialize)]
pub struct NewThreadRequest {
    pub title: String,
    pub content: String,
    pub club_id: i32,
}

#[derive(FromRow, Serialize)]
pub struct Thread {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub created_at: i32,
    pub club_id: i32,
    pub likes: i32,
}
#[derive(FromRow, Serialize)]
pub struct Comment {
    pub id: i32,
    pub content: String,
    pub user_name: String,
    pub likes: i32,
    pub created_at: i32,
}

pub async fn threads(State(state): State<AppState>) -> Json<Vec<Thread>> {
    Json(
        sqlx::query_as::<_, Thread>(
            "SELECT (id, title, content, created_at, club_id, likes) FROM thread",
        )
        .fetch_all(&state.connection)
        .await
        .unwrap(),
    )
}

pub async fn comments(
    State(state): State<AppState>,
    comment_request: Json<CommentRequest>,
) -> Json<Vec<Comment>> {
    Json(
        sqlx::query_as::<_, Comment>(
            "SELECT 
            (comment.id, comment.content, user_profile.user_name, comment.likes, comment.created_at) 
            FROM comment 
            INNER JOIN user_profile ON comment.user_id = user_profile.id
            WHERE thread_id = $1",
        )
        .bind(comment_request.thread_id)
        .fetch_all(&state.connection)
        .await
        .unwrap(),
    )
}

// TODO
pub async fn create_thread(
    State(state): State<AppState>,
    thread_data: Json<NewThreadRequest>,
) -> Json<bool> {
    match sqlx::query(
        "INSERT INTO thread (title, content, club_id) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&thread_data.title)
    .bind(&thread_data.content)
    .bind(thread_data.club_id)
    .fetch_one(&state.connection)
    .await
    {
        Ok(thread_id) => Json(true),
        Err(err) => Json(false),
    }
}
