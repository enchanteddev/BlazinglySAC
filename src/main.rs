use std::{
    collections::HashMap,
    env::{self, args},
};

use axum::{
    http::{HeaderMap, HeaderValue},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use local_ip_address::local_ip;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

mod models;

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

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
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .with_state(state);

    if network {
        println!("listening on http://{}", listener.local_addr().unwrap());
    }
    println!("listening on {}", base_url);

    axum::serve(listener, app).await.unwrap();
}