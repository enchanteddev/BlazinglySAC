use std::io::Cursor;

use axum::body::Bytes;
use axum::extract::{Multipart, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use sqlx::PgConnection;

use crate::models::AppState;

#[derive(Deserialize)]
struct ViewRequest {
    hash: String,
}

#[derive(Debug)]
enum FileHandleError {
    InvalidImage,
    FailedToWriteAsWebP,
    #[allow(dead_code)]
    SQLError(sqlx::Error),
}

#[derive(Deserialize)]
struct AttachmentRequest {
    id: i32,
    attachment_type: AttachmentType
}

#[derive(Deserialize)]
enum AttachmentType {
    Announcement,
    Thread,
    Event,
}

enum AttachmentId {
    Announcement(i32),
    Thread(i32),
    Event(i32),
}

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/upload/", post(upload_file))
        .route("/view", get(view_file))
        .route("/attachment", get(view_attachment_from_id))
        .with_state(state)
}

async fn handle_upload(
    connection: &mut PgConnection,
    name: String,
    data: Bytes,
) -> Result<i32, FileHandleError> {
    let extension = name.split('.').last().unwrap_or("file"); // no file extension was given
    let original_hash = blake3::hash(&data).to_string();

    match sqlx::query_scalar!(
        "SELECT id FROM upload WHERE original_hash = $1",
        original_hash
    )
    .fetch_one(connection.as_mut())
    .await
    {
        Ok(image_id) => return Ok(image_id),
        Err(err) => {
            println!("Error: {err}")
        }
    };

    let (compressed_hash, compressed_data, file_type) =
        if extension == "png" || extension == "jpg" || extension == "jpeg" {
            let Ok(image) = image::load_from_memory(&data) else {
                return Err(FileHandleError::InvalidImage);
            };

            let mut new_data = Cursor::new(Vec::<u8>::new());

            image
                .write_to(&mut new_data, image::ImageFormat::WebP)
                .map_err(|_| FileHandleError::FailedToWriteAsWebP)?;

            let compressed = new_data.into_inner();

            (blake3::hash(&compressed).to_string(), compressed, "webp")
        } else {
            (original_hash.clone(), data.to_vec(), extension)
        };

    match sqlx::query_scalar!(
        "SELECT id FROM upload WHERE compressed_hash = $1",
        compressed_hash.clone()
    )
    .fetch_one(connection.as_mut())
    .await
    {
        Ok(image_id) => return Ok(image_id),
        Err(_) => {}
    };

    let image_id = match sqlx::query_scalar!(
        "INSERT INTO upload (file_type, blob, original_hash, compressed_hash) 
            VALUES ($1, $2, $3, $4) RETURNING id",
        file_type,
        compressed_data,
        original_hash,
        compressed_hash
    )
    .fetch_one(connection)
    .await
    {
        Ok(image_id) => image_id,
        Err(e) => return Err(FileHandleError::SQLError(e)),
    };

    Ok(image_id)
}

async fn bind_attachment(
    attachment_id: AttachmentId,
    media_id: i32,
    connection: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    match attachment_id {
        AttachmentId::Announcement(aid) => {
            match sqlx::query!(
                "INSERT INTO announcement_media (announcement_id, media_id) VALUES ($1, $2)",
                aid,
                media_id
            )
            .execute(connection)
            .await
            {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        }
        AttachmentId::Thread(tid) => {
            match sqlx::query!(
                "INSERT INTO thread_media (thread_id, media_id) VALUES ($1, $2)",
                tid,
                media_id
            )
            .execute(connection)
            .await
            {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        }
        AttachmentId::Event(eid) => {
            match sqlx::query!(
                "INSERT INTO event_media (event_id, media_id) VALUES ($1, $2)",
                eid,
                media_id
            )
            .execute(connection)
            .await
            {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        }
    }
}

async fn upload_file(State(state): State<AppState>, mut multipart: Multipart) -> String {
    let mut attachment_id: Option<AttachmentId> = None;
    let mut file_name: Option<String> = None;
    let mut file_data: Option<Bytes> = None;
    while let Some(field) = match multipart.next_field().await {
        Ok(field) => field,
        Err(err) => return format!("Failed to get field: {}", err),
    } {
        let Some(name) = field.name() else {
            continue;
        };
        let name = name.to_string();
        if name == "file" {
            let fname = match field.file_name() {
                Some(fname) => {
                    file_name = Some(fname.to_string());
                    fname.to_string()
                }
                None => {
                    return String::from("File name not found in headers");
                }
            };
            // let file_name = file_name.to_string();
            let datalen = match field.bytes().await {
                Ok(data) => {
                    let length = data.len();
                    file_data = Some(data);
                    length
                }
                Err(_) => {
                    return String::from("Failed to get data");
                }
            };
            println!("Length of `{}` is {} bytes", fname, datalen);
        } else if name == "announcement_id" || name == "thread_id" || name == "event_id" {
            match field.text().await {
                Ok(att_id) => {
                    let Ok(aid) = att_id.parse::<i32>() else {
                        return format!("Failed to parse '{name}' as a number");
                    };
                    match name.as_str() {
                        "announcement_id" => attachment_id = Some(AttachmentId::Announcement(aid)),
                        "thread_id" => attachment_id = Some(AttachmentId::Thread(aid)),
                        "event_id" => attachment_id = Some(AttachmentId::Event(aid)),
                        _ => {
                            unreachable!()
                        }
                    }
                }
                Err(_) => {
                    return format!("Failed to get '{}'", name.clone());
                }
            };
        }
    }

    let Some(fname) = file_name else {
        return String::from("File name not found");
    };
    let Some(data) = file_data else {
        return String::from("File data not found");
    };
    let Some(att_id) = attachment_id else {
        return String::from("Attachment ID not found");
    };
    tokio::spawn(async move {
        let mut txn = state
            .connection
            .begin()
            .await
            .expect("Failed to start transaction");
        let media_id = match handle_upload(&mut *txn, fname, data).await {
            Ok(media_id) => media_id,
            Err(e) => {
                println!("Upload task failed with error: {e:?}");
                return;
            }
        };
        println!("Compression task completed");
        match bind_attachment(att_id, media_id, &mut *txn).await {
            Ok(_) => {}
            Err(e) => {
                println!("Binding task failed with error: {e:?}");
                return;
            }
        }
        txn.commit().await.expect("Failed to commit transaction");
        println!("Upload task completed");
    });
    String::from("Upload done.")
}

async fn view_file(
    Query(query): Query<ViewRequest>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
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

async fn view_attachment_from_id(
    State(state): State<AppState>,
    Query(attachment_id): Query<AttachmentRequest>,
) -> Redirect {
    let attachment_id = match attachment_id .attachment_type{
        AttachmentType::Announcement => AttachmentId::Announcement(attachment_id.id),
        AttachmentType::Thread => AttachmentId::Thread(attachment_id.id),
        AttachmentType::Event => AttachmentId::Event(attachment_id.id),
    };
    let hash = match attachment_id {
        AttachmentId::Announcement(aid) => sqlx::query_scalar!(
            "
                SELECT upload.compressed_hash FROM announcement_media
                INNER JOIN upload ON upload.id = announcement_media.media_id 
                WHERE announcement_id = $1
                ",
            aid
        )
        .fetch_one(&state.connection)
        .await
        .ok(),
        AttachmentId::Event(eid) => sqlx::query_scalar!(
            "
                SELECT upload.compressed_hash FROM event_media
                INNER JOIN upload ON upload.id = event_media.media_id 
                WHERE event_id = $1
                ",
            eid
        )
        .fetch_one(&state.connection)
        .await
        .ok(),
        AttachmentId::Thread(tid) => sqlx::query_scalar!(
            "
                SELECT upload.compressed_hash FROM thread_media
                INNER JOIN upload ON upload.id = thread_media.media_id 
                WHERE thread_id = $1
                ",
            tid
        )
        .fetch_one(&state.connection)
        .await
        .ok(),
    };

    match hash {
        Some(hash) => Redirect::to(&format!("/media/view?hash={hash}")),
        None => Redirect::to("/404"),
    }
}
