use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::prelude::FromRow;

use crate::{auth::Claims, models::AppState};

#[derive(Deserialize)]
pub struct CommentRequest {
    pub thread_id: i32,
}

#[derive(FromRow, Deserialize)]
pub struct NewThreadRequest {
    pub title: String,
    pub content: String,
    pub club_id: i32,
}

#[derive(FromRow, Deserialize)]
pub struct NewCommentRequest {
    pub content: String,
    pub thread_id: i32,
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

pub enum StatusResponse {
    Success,
    ServerError,
    UserError(String),
}

impl IntoResponse for StatusResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            StatusResponse::Success => (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                })),
            )
                .into_response(),
            StatusResponse::ServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": "Server Error",
                })),
            )
                .into_response(),
            StatusResponse::UserError(err) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": err,
                })),
            )
                .into_response(),
        }
    }
}

pub async fn threads(State(state): State<AppState>) -> Json<Vec<Thread>> {
    Json(
        sqlx::query_as::<_, Thread>(
            "SELECT id, title, content, created_at, club_id, likes FROM thread",
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
            comment.id, comment.content, user_profile.user_name, comment.likes, comment.created_at
            FROM comment 
            INNER JOIN user_profile ON comment.user_email = user_profile.email
            WHERE thread_id = $1",
        )
        .bind(comment_request.thread_id)
        .fetch_all(&state.connection)
        .await
        .unwrap(),
    )
}

pub async fn create_thread(
    claim: Claims,
    State(state): State<AppState>,
    thread_data: Json<NewThreadRequest>,
) -> StatusResponse {
    // Check if user is in club
    let user_id = claim.id;
    let club_id = thread_data.club_id;
    let Ok(privilege_level) = sqlx::query_scalar::<_, i32>(
        "
            SELECT privilege_level FROM membership
            WHERE user_id = $1 AND club_id = $2
            LIMIT 1
        ",
    )
    .bind(user_id)
    .bind(club_id)
    .fetch_one(&state.connection)
    .await
    else {
        return StatusResponse::ServerError;
    };

    if privilege_level <= 1 {
        return StatusResponse::UserError(
            "You are not allowed to create threads in this club".to_string(),
        );
    }

    match sqlx::query(
        "INSERT INTO thread (title, content, club_id) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&thread_data.title)
    .bind(&thread_data.content)
    .bind(thread_data.club_id)
    .fetch_one(&state.connection)
    .await
    {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    }
}

pub async fn create_comment(
    claim: Claims,
    State(state): State<AppState>,
    comment_request: Json<NewCommentRequest>,
) -> StatusResponse {
    // Dont check if user is in club
    let user_id = claim.id;
    let thread_id = comment_request.thread_id;

    match sqlx::query("INSERT INTO comment (content, thread_id, user_id) VALUES ($1, $2, $3)")
        .bind(&comment_request.content)
        .bind(thread_id)
        .bind(user_id)
        .execute(&state.connection)
        .await
    {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    }
}
