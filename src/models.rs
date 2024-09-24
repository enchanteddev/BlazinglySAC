use std::collections::HashMap;

use sqlx::{migrate::MigrateDatabase, postgres::PgPoolOptions, Pool, Postgres};

const DB_URL: &str = "postgresql://krawat:pwd@localhost:5432/blazinglysac";

#[derive(Clone)]
pub struct AppState {
    pub connection: Pool<Postgres>,
    pub env_vars: HashMap<String, String>,
}

pub async fn get_connection() -> Pool<Postgres> {
    if !Postgres::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Postgres::create_database(DB_URL).await {
            Ok(_) => println!("Created new db successfully"),
            Err(error) => panic!("Failed to create Database. Error: {}", error),
        }
    } else {
        println!("Database already exists");
    }

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(DB_URL)
        .await
        .unwrap();

    sqlx::migrate!().run(&db).await.unwrap();
    println!("Ran migrations");
    db
}
