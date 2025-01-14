use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::Claims, models::AppState, thread_comment::StatusResponse, validation::check_emails,
};

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/list", get(list_clubs))
        .route("/list_my", get(list_my_clubs))
        .route("/create", post(create_club))
        .route("/update", post(update_club))
        .route("/join", post(join_club))
        .route("/view_applications", get(view_club_application))
        .route("/accept_application", post(accept_club_join_application))
        .with_state(state)
}

#[derive(Serialize)]
struct ClubBasic {
    id: i32,
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
struct ClubJoinApplicationRequest {
    club_id: i32,
    message: Option<String>,
}

#[derive(Deserialize)]
struct ClubViewApplicationRequest {
    club_id: i32,
}

#[derive(Deserialize)]
struct ClubAcceptApplicationRequest {
    application_id: i32,
}

#[derive(Serialize)]
struct ClubApplicationRequest {
    id: i32,
    user_name: String,
    user_email: String,
    message: Option<String>,
    created_at: DateTime<Utc>,
    accepted: bool,
    accepted_at: Option<DateTime<Utc>>,
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
            "SELECT club.id, club.name, description, council.name as council_name FROM club INNER JOIN council ON club.council_id = council.id",
        )
        .fetch_all(&state.connection)
        .await
        .unwrap(),
    )
}

async fn list_my_clubs(claims: Claims, State(state): State<AppState>) -> Json<Vec<ClubBasic>> {
    let user_id = claims.id;
    Json(
        sqlx::query_as!(
            ClubBasic,
            "
            SELECT club.id, club.name, description, council.name as council_name 
            FROM club INNER JOIN council ON club.council_id = council.id
            INNER JOIN membership on membership.club_id = club.id WHERE membership.user_id = $1;
            ",
            user_id
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

async fn join_club(
    claims: Claims,
    State(state): State<AppState>,
    Json(club_join_request): Json<ClubJoinApplicationRequest>,
) -> StatusResponse {
    let user_id = claims.id;
    let Ok(membership) = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM membership WHERE user_id = $1 AND club_id = $2",
        user_id,
        club_join_request.club_id
    )
    .fetch_one(&state.connection)
    .await
    else {
        return StatusResponse::ServerError;
    };

    let is_member = membership.unwrap_or(0) > 0;
    if is_member {
        return StatusResponse::UserError("You are already a member of this club".to_string());
    }

    match sqlx::query!(
        "INSERT INTO club_application (club_id, user_id, message) VALUES ($1, $2, $3)",
        club_join_request.club_id,
        user_id,
        club_join_request.message
    )
    .execute(&state.connection)
    .await
    {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    }
}

async fn view_club_application(
    claims: Claims,
    State(state): State<AppState>,
    Query(club_view_application_request): Query<ClubViewApplicationRequest>,
) -> Result<Json<Vec<ClubApplicationRequest>>, StatusResponse> {
    let user_email = claims.email;
    let club_id = club_view_application_request.club_id;
    let club_head_emails =
        sqlx::query_scalar!("SELECT club_head_emails FROM club WHERE id = $1", club_id)
            .fetch_one(&state.connection)
            .await
            .unwrap();

    if !club_head_emails.contains(&user_email) {
        return Err(StatusResponse::UserError(
            "You are not a club head".to_string(),
        ));
    }

    let applications = sqlx::query_as!(
        ClubApplicationRequest,
        "SELECT 
            club_application.id as \"id\", user_profile.name as \"user_name\", user_profile.email as \"user_email\", 
            message, created_at, accepted, accepted_at
        FROM club_application 
        INNER JOIN user_profile ON user_profile.id = club_application.user_id
        WHERE club_id = $1",
        club_id
    )
    .fetch_all(&state.connection)
    .await
    .unwrap();

    Ok(Json(applications))
}

async fn accept_club_join_application(
    claims: Claims,
    State(state): State<AppState>,
    Json(club_accept_application_request): Json<ClubAcceptApplicationRequest>,
) -> StatusResponse {
    let user_email = claims.email;
    let application_id = club_accept_application_request.application_id;
    let Ok(club_id) = sqlx::query_scalar!(
        "SELECT club_id FROM club_application WHERE id = $1",
        application_id
    )
    .fetch_one(&state.connection)
    .await
    else {
        return StatusResponse::ServerError;
    };

    let Ok(club_head_emails) =
        sqlx::query_scalar!("SELECT club_head_emails FROM club WHERE id = $1", club_id)
            .fetch_one(&state.connection)
            .await
    else {
        return StatusResponse::ServerError;
    };

    if !club_head_emails.contains(&user_email) {
        return StatusResponse::UserError("You are not a club head".to_string());
    }

    match sqlx::query!(
        "UPDATE club_application SET 
            accepted = TRUE,
            accepted_at = (now() at time zone 'utc')
        WHERE id = $1",
        application_id
    )
    .execute(&state.connection)
    .await
    {
        Ok(_) => StatusResponse::Success,
        Err(err) => StatusResponse::UserError(err.to_string()),
    }
}
