use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    prelude::FromRow,
    types::time::PrimitiveDateTime,
};

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
    #[serde(serialize_with = "pdt_to_unixtime")]
    pub created_at: PrimitiveDateTime,
    pub club_id: i32,
    pub likes: i32,
}
#[derive(FromRow, Serialize)]
pub struct Comment {
    pub id: i32,
    pub content: String,
    pub user_name: String,
    pub likes: i32,
    #[serde(serialize_with = "pdt_to_unixtime")]
    pub created_at: PrimitiveDateTime,
}

fn pdt_to_unixtime<S>(ndt: &PrimitiveDateTime, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_i64(ndt.assume_utc().unix_timestamp())
}

#[derive(FromRow, Deserialize)]
pub struct LikeRequest {
    pub id: i32,
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
        sqlx::query_as!(
            Thread,
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
        sqlx::query_as!(Comment,
            "SELECT 
            comment.id, comment.content, user_profile.name as user_name, comment.likes, comment.created_at
            FROM comment 
            INNER JOIN user_profile ON comment.user_id = user_profile.id
            WHERE thread_id = $1",
        comment_request.thread_id)
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
    let Ok(privilege_level) = sqlx::query_scalar!(
        "
            SELECT privilege_level FROM membership
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
            "You are not allowed to create threads in this club".to_string(),
        );
    }

    match sqlx::query!(
        "INSERT INTO thread (title, content, club_id) VALUES ($1, $2, $3) RETURNING id",
        &thread_data.title,
        &thread_data.content,
        thread_data.club_id
    )
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

    match sqlx::query!(
        "INSERT INTO comment (content, thread_id, user_id) VALUES ($1, $2, $3)",
        &comment_request.content,
        thread_id,
        user_id
    )
    .execute(&state.connection)
    .await
    {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    }
}

pub async fn like_thread(
    claim: Claims,
    State(state): State<AppState>,
    thread_id: Json<LikeRequest>,
) -> StatusResponse {
    let user_id = claim.id;
    let mut transaction = state.connection.begin().await.unwrap();

    match sqlx::query!(
        "INSERT INTO thread_likes (user_id, thread_id) VALUES ($1, $2)",
        user_id,
        thread_id.id
    )
    .execute(&mut *transaction)
    .await
    {
        Ok(_) => {}
        Err(err) => match err {
            sqlx::Error::Database(err) if err.is_unique_violation() => {
                return StatusResponse::UserError("You already liked this thread".to_string());
            }
            _ => return StatusResponse::ServerError,
        },
    };

    let completion_status = match sqlx::query!(
        "UPDATE thread SET likes = likes + 1 WHERE id = $1",
        thread_id.id
    )
    .execute(&mut *transaction)
    .await
    {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    };

    transaction.commit().await.unwrap();

    completion_status
}

pub async fn like_comment(
    claim: Claims,
    State(state): State<AppState>,
    comment_id: Json<LikeRequest>,
) -> StatusResponse {
    let user_id = claim.id;
    let mut transaction = state.connection.begin().await.unwrap();

    match sqlx::query!(
        "INSERT INTO comment_likes (user_id, comment_id) VALUES ($1, $2)",
        user_id,
        comment_id.id
    )
    .execute(&mut *transaction)
    .await
    {
        Ok(_) => {}
        Err(err) => match err {
            sqlx::Error::Database(err) if err.is_unique_violation() => {
                return StatusResponse::UserError("You already liked this comment".to_string());
            }
            _ => return StatusResponse::ServerError,
        },
    };

    let completion_status = match sqlx::query!(
        "UPDATE comment SET likes = likes + 1 WHERE id = $1",
        comment_id.id
    )
    .execute(&mut *transaction)
    .await
    {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    };

    transaction.commit().await.unwrap();

    completion_status
}
