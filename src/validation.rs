use sqlx::{Pool, Postgres};

pub async fn check_emails(emails: &[String], connection: Pool<Postgres>) -> Result<bool, String> {
    // Checks if all emails are present in the database. 
    // Relies on the UNIQUE constraint on email field in user_profile table

    let count = emails.len();
    let count_in_db = sqlx::query_scalar!(
        "SELECT COUNT(*) AS \"count!\" FROM user_profile WHERE email = ANY($1)",
        emails
    )
    .fetch_one(&connection)
    .await
    .map_err(|e| e.to_string())?;

    Ok(count_in_db == count as i64)
}
