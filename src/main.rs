use std::{
    collections::HashMap,
    env::{self, args},
};

use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use local_ip_address::local_ip;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

mod models;
mod views;
mod thread_comment;
mod auth;
mod club;
mod council;
mod file_uploads;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    match dotenv() {
        Err(load_error) => println!("Failed to load .env, Error: {}", load_error),
        Ok(path) => println!("Loaded .env at location: {}", path.display()),
    }

    let env_vars: HashMap<String, String> = env::vars().collect();

    let args: Vec<String> = args().collect();
    let network = !(args.len() < 2 || args[1] != "host");
    let port = {
        if args.len() < 2 {
            5000
        } else {
            let port: u32 = args[1].parse().unwrap_or_else(|_| {
                let Some(arg2) = args.get(2) else {
                    return 5000;
                };
                arg2.parse().unwrap_or(5000)
            });
            port
        }
    };
    let address = if network { "0.0.0.0" } else { "127.0.0.1" };

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", address, port))
        .await
        .unwrap();

    let base_url = if network {
        format!("http://{}:{}", local_ip().unwrap(), port)
    } else {
        format!("http://{}", listener.local_addr().unwrap())
    };

    let state = models::AppState {
        connection: models::get_connection().await,
        env_vars,
    };
    let app = Router::new()
        .route("/home/announcements", get(views::announcements))
        .route("/conversation/threads", get(thread_comment::threads)) 
        .route("/conversation/threads/like/", post(thread_comment::like_thread)) 
        .route("/conversation/threads/new/", post(thread_comment::create_thread))
        .route("/conversation/comments", get(thread_comment::comments))
        .route("/conversation/comments/like/", post(thread_comment::like_comment))
        .route("/conversation/comments/new/", post(thread_comment::create_comment))
        .route("/auth/login/", post(auth::login))
        .route("/auth/register/", post(auth::register))
        .route("/auth/private", get(auth::private))
        .nest("/club", club::routes(state.clone()))
        .nest("/council", council::routes(state.clone()))
        .nest("/media", file_uploads::routes(state.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .with_state(state);

    if network {
        println!("listening on http://{}", listener.local_addr().unwrap());
    }
    println!("listening on {}", base_url);

    axum::serve(listener, app).await.unwrap();
}