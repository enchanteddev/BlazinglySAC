use sqlx::{migrate::MigrateDatabase, postgres::PgPoolOptions, Pool, Postgres};

#[derive(Clone)]
pub struct AppState {
    pub connection: Pool<Postgres>
}

pub async fn get_connection(db_url: &str) -> Pool<Postgres> {
    if !Postgres::database_exists(db_url).await.unwrap_or(false) {
        println!("Creating database {}", db_url);
        match Postgres::create_database(db_url).await {
            Ok(_) => println!("Created new db successfully"),
            Err(error) => panic!("Failed to create Database. Error: {}", error),
        }
    } else {
        println!("Database already exists");
    }

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(db_url)
        .await
        .unwrap();

    sqlx::migrate!().run(&db).await.unwrap();
    println!("Ran migrations");
    db
}
