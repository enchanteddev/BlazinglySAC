use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, RequestPartsExt, Router,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{prelude::FromRow, Error};
use std::sync::LazyLock;
use std::time::SystemTime;

use crate::models::AppState;

// static KEYS: Lazy<Keys> = Lazy::new(|| {
//     let secret = "JWT_SECRET".to_string();
//     Keys::new(secret.as_bytes())
// });

// encoding/decoding keys - set in the static `once_cell` above

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/private", get(private))
        .route("/login/", post(login))
        .route("/register/", post(register))
        .with_state(state)
}

struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(FromRow, Serialize)]
struct UserProfile {
    id: i32,
    name: String,
    email: String,
    password_hash: String,
}

// the JWT claim
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub exp: usize,
}

// the response that we pass back to HTTP client once successfully authorised
#[derive(Debug, Serialize)]
struct AuthBody {
    access_token: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct AuthPayload {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct RegisterAuthPayload {
    name: String,
    email: String,
    password: String,
}

// error types for auth errors
#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
    UserAlreadyExists,
    InternalServerError,
}

// implement a method to create a response type containing the JWT
impl AuthBody {
    fn new(access_token: String) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
        }
    }
}

// implement FromRequestParts for Claims (the JWT struct)
// FromRequestParts allows us to use Claims without consuming the request
#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;
        // Decode the user data
        let token_data = decode::<Claims>(bearer.token(), &KEYS.decoding, &Validation::default())
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}

// implement IntoResponse for AuthError so we can use it as an Axum response type
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InternalServerError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Interal Server error")
            }
            AuthError::UserAlreadyExists => (StatusCode::BAD_REQUEST, "User Already exists"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let secret = "JWT_SECRET".to_string();
    Keys::new(secret.as_bytes())
});

async fn private(claims: Claims) -> Result<String, AuthError> {
    Ok(format!(
        "Welcome to the protected area :)\nYour data:\n{claims:?}",
    ))
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthBody>, AuthError> {
    if payload.email.is_empty() || payload.password.is_empty() {
        return Err(AuthError::MissingCredentials);
    }

    let user_profile = sqlx::query_as!(
        UserProfile,
        "SELECT id, name, email, password as password_hash FROM user_profile WHERE email = $1",
        payload.email
    )
    .fetch_one(&state.connection)
    .await
    .map_err(|e| {
        println!("Error: {e}");
        AuthError::WrongCredentials
    })?;

    let password_is_correct =
        verify(payload.password, &user_profile.password_hash).expect("Failed to verify password");

    if !password_is_correct {
        return Err(AuthError::WrongCredentials);
    }

    // add 5 minutes to current unix epoch time as expiry date/time
    let exp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 24 * 3600 * 7;

    let claims = Claims {
        id: user_profile.id,
        name: user_profile.name,
        email: user_profile.email,
        exp: usize::try_from(exp).unwrap(),
    };
    // Create the authorization token
    let token = encode(&Header::default(), &claims, &KEYS.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    // Send the authorized token
    Ok(Json(AuthBody::new(token)))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterAuthPayload>,
) -> Result<Json<AuthBody>, AuthError> {
    if payload.email.is_empty() || payload.password.is_empty() || payload.name.is_empty() {
        return Err(AuthError::MissingCredentials);
    }

    let hashed_password = hash(payload.password, DEFAULT_COST).expect("Hashing Failed");

    // now create the user
    let user_id = match sqlx::query_scalar!(
        "INSERT INTO user_profile (name, email, password) VALUES ($1, $2, $3) RETURNING id",
        &payload.name,
        &payload.email,
        hashed_password
    )
    .fetch_one(&state.connection)
    .await
    {
        Ok(user_id) => user_id,
        Err(e) => match e {
            Error::Database(db_error) if db_error.is_unique_violation() => {
                return Err(AuthError::UserAlreadyExists);
            }
            _ => {
                println!("{e}");
                return Err(AuthError::InternalServerError);
            }
        },
    };

    // add 5 minutes to current unix epoch time as expiry date/time
    let exp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 24 * 3600 * 7;

    let claims = Claims {
        id: user_id,
        name: payload.name,
        email: payload.email,
        exp: usize::try_from(exp).unwrap(),
    };
    // Create the authorization token
    let token = encode(&Header::default(), &claims, &KEYS.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    // Send the authorized token
    Ok(Json(AuthBody::new(token)))
}
