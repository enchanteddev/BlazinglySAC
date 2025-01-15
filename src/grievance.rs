use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{auth::Claims, models::AppState, thread_comment::StatusResponse};

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/create", post(create_grievance))
        .route("/list", get(list_grievances))
        .with_state(state)
}

#[derive(Deserialize)]
struct GrievanceCreate {
    email: String,
    grievance: String,
}

#[derive(Serialize)]
struct GrievanceView {
    id: i32,
    email: String,
    grievance: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn create_grievance(
    State(state): State<AppState>,
    Json(grievance_data): Json<GrievanceCreate>,
) -> StatusResponse {
    match sqlx::query!(
        "INSERT INTO website_grievance (email, grievance) VALUES ($1, $2)",
        grievance_data.email,
        grievance_data.grievance
    )
    .execute(&state.connection)
    .await
    {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    }
}

async fn list_grievances(
    claims: Claims,
    State(state): State<AppState>,
) -> Json<Vec<GrievanceView>> {
    // Only proceed if user is an admin
    if sqlx::query_scalar!("SELECT id FROM admin WHERE id = $1", claims.id)
        .fetch_optional(&state.connection)
        .await
        .unwrap()
        .is_none()
    {
        return Json(vec![]);
    }

    Json(
        sqlx::query_as!(
            GrievanceView,
            "SELECT id, email, grievance, created_at FROM website_grievance ORDER BY created_at DESC"
        )
        .fetch_all(&state.connection)
        .await
        .unwrap_or_default(),
    )
}
