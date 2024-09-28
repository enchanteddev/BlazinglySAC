use std::io::Cursor;

use axum::body::Bytes;
use axum::extract::{Multipart, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use sqlx::{Pool, Postgres};

use crate::models::AppState;

#[derive(Deserialize)]
struct ViewRequest {
    hash: String,
}

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/upload/", post(upload_file))
        .route("/view", get(view_file))
        .with_state(state)
}

async fn handle_upload(connection: &Pool<Postgres>, name: String, data: Bytes) {
    let extension = name.split('.').last().unwrap_or("file"); // no file extemsion was given
    let original_hash = blake3::hash(&data).to_string();

    let does_image_exist =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM upload WHERE original_hash = $1")
            .bind(original_hash.clone())
            .fetch_one(connection)
            .await
            .unwrap();

    if does_image_exist > 0 {
        return;
    }

    let (compressed_hash, compressed_data, file_type) =
        if extension == "png" || extension == "jpg" || extension == "jpeg" {
            let image = image::load_from_memory(&data).unwrap();
            let mut new_data = Cursor::new(Vec::<u8>::new());
            image
                .write_to(&mut new_data, image::ImageFormat::WebP)
                .unwrap();
            let compressed = new_data.into_inner();

            (blake3::hash(&compressed).to_string(), compressed, "webp")
        } else {
            (original_hash.clone(), data.to_vec(), extension)
        };

    sqlx::query("INSERT INTO upload (file_type, blob, original_hash, compressed_hash) VALUES ($1, $2, $3, $4)")
        .bind(file_type)
        .bind(compressed_data)
        .bind(original_hash)
        .bind(compressed_hash)
        .execute(connection)
        .await
        .unwrap();
}

async fn upload_file(State(state): State<AppState>, mut multipart: Multipart) -> String {
    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(err) => return format!("Failed to get field: {}", err),
    } {
        let Some(name) = field.name() else {
            continue;
        };
        if name == "file" {
            let Some(file_name) = field.file_name() else {
                return String::from("File name not found in headers");
            };
            let file_name = file_name.to_string();
            let Ok(data) = field.bytes().await else {
                return String::from("Failed to get data");
            };
            println!("Length of `{}` is {} bytes", file_name, data.len());
            tokio::spawn(async move {
                handle_upload(&state.connection, file_name, data).await;
                println!("Upload task completed");
            });
            break;
        }
    }
    String::from("Upload done.")
}

async fn view_file(
    Query(query): Query<ViewRequest>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    println!("what");
    let Ok((file_bytes, file_type)) = sqlx::query_scalar::<_, (Vec<u8>, String)>(
        "SELECT (blob, file_type) FROM upload WHERE compressed_hash = $1",
    )
    .bind(query.hash)
    .fetch_one(&state.connection)
    .await
    else {
        return Err(StatusCode::NOT_FOUND);
    };

    let mut headers = HeaderMap::new();
    headers.insert("Cache-Control", "max-age=31536000".parse().unwrap());

    if file_type == "webp" {
        headers.insert("Content-Type", "image/webp".parse().unwrap());
    } else {
        headers.insert("Content-Type", file_type.parse().unwrap());
    }

    Ok((headers, file_bytes))
}
