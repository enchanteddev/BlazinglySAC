use axum::{extract::State, routing::get, Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, types::chrono::DateTime};

use crate::models::AppState;

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/bus_from", get(get_next_bus_from_starting_point))
        .with_state(state)
}

#[derive(Serialize, Deserialize)]
struct StartPointRequest {
    start_point: String,
}

#[derive(FromRow, Deserialize, Serialize)]
struct Bus {
    stops: Vec<String>,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

async fn get_next_bus_from_starting_point(
    State(state): State<AppState>,
    start_point_request: Json<StartPointRequest>,
) -> Json<Vec<Bus>> {
    // TODO GET NEXT BUS BASED ON CURRENT TIME!
    Json(
        sqlx::query_as!(
                Bus,
                "SELECT stops, start_time, end_time FROM transportation WHERE stops[1] = $1",
                start_point_request.start_point
            )
            .fetch_all(&state.connection)
            .await
            .unwrap(),
    )
}
