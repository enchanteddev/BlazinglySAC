use axum::{
    async_trait,
    extract::{FromRequestParts, Path, State},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, RequestPartsExt, Router,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use mail_send::{mail_builder::MessageBuilder, SmtpClientBuilder};
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
        .route("/whoami", get(whoami))
        .route("/verify/:token", get(verify_user))
        .route("/reverify/", post(send_verification))
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
    active: bool,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: i32,
    pub name: String,
    pub email: String,
}

// the response that we pass back to HTTP client once successfully authorised
#[derive(Debug, Serialize)]
struct AuthBody {
    access_token: String,
    token_type: String,
}

#[derive(Debug, Serialize)]
struct ResponseBody {
    message: String,
}

#[derive(Debug, Deserialize)]
struct AuthPayload {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct VerificationRequestPayload {
    email: String,
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
    UserAlreadyVerified,
    InternalServerError,
    UserNotActive,
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
            AuthError::UserAlreadyVerified => (StatusCode::BAD_REQUEST, "User Already Verified"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::UserNotActive => (StatusCode::BAD_REQUEST, "User not Active"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

static KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let secret = "JWT_SECRET".to_string(); // TODO CHANGE KEY BEFORE LAUNCH!
    Keys::new(secret.as_bytes())
});

static APPPASS: LazyLock<String> = LazyLock::new(|| {
    "Key".to_string() // TODO get the key from env
});

async fn send_email(subject: &str, body: &str, to: Vec<&str>) -> Result<(), AuthError> {
    // Build a simple multipart message
    let message = MessageBuilder::new()
        .from("sacbackend25@gmail.com")
        .to(to)
        .subject(subject)
        .html_body(body);

    // Connect to the SMTP submissions port, upgrade to TLS and
    // authenticate using the provided credentials.
    SmtpClientBuilder::new("smtp.gmail.com", 587)
        .implicit_tls(false)
        .credentials(("sacbackend25@gmail.com", APPPASS.as_str()))
        .connect()
        .await
        .map_err(|_| AuthError::InternalServerError)?
        .send(message)
        .await
        .map_err(|_| AuthError::InternalServerError)?;

    Ok(())
}

async fn send_verification(
    State(state): State<AppState>,
    Json(payload): Json<VerificationRequestPayload>,
) -> Result<Json<ResponseBody>, AuthError> {
    if payload.email.is_empty() {
        return Err(AuthError::MissingCredentials);
    }

    let user_profile = sqlx::query_as!(
        UserProfile,
        "SELECT id, name, email, active, password as password_hash FROM user_profile WHERE email = $1",
        payload.email
    )
    .fetch_one(&state.connection)
    .await
    .map_err(|e| {
        println!("Error: {e}");
        AuthError::WrongCredentials
    })?;

    // checking if user is already active
    if user_profile.active {
        return Err(AuthError::UserAlreadyVerified);
    }

    let token = create_token(&user_profile).await?;

    send_email(
        "Login to SAC",
        get_html(&token).as_str(),
        vec![payload.email.as_str()],
    )
    .await?;

    Ok(Json(ResponseBody {
        message: format!(
            "Verification Link sent Successfully to {}",
            user_profile.email
        ),
    }))
}

fn get_html(token: &str) -> String{
    return format!(r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="UTF-8" />
    <title>Email</title>
  </head>
  <body style="font-family: Arial, sans-serif; background-color: #f4f4f4; padding: 20px;">
    <table width="100%" cellpadding="0" cellspacing="0">
      <tr>
        <td align="center">
          <table width="600" cellpadding="0" cellspacing="0" style="background-color: #ffffff; padding: 40px; border-radius: 8px;">
            <tr>
              <td align="center" style="font-size: 24px; font-weight: bold; color: #333333;">
                Hello,<br/>
                Please verify your email to activate your account for the <a href="http://0.0.0.0:3000/">SAC-website</a>
              </td>
            </tr>
            <tr>
              <td align="center" style="padding: 20px 0; color: #555555;">
                Click the button below to verify your email address:
              </td>
            </tr>
            <tr>
              <td align="center">
                <a href="http://127.0.0.1:5000/auth/verify/{token}"
                   style="background-color: #4CAF50; color: white; padding: 14px 28px;
                          text-decoration: none; font-size: 16px; border-radius: 5px; display: inline-block;">
                  Verify Email
                </a>
              </td>
            </tr>
            <tr>
              <td align="center" style="padding-top: 20px; font-size: 12px; color: #888888;">
                If you did not request this, please ignore this email.
              </td>
            </tr>
          </table>
        </td>
      </tr>
    </table>
  </body>
</html>
"#)
}

async fn create_token(user_profile: &UserProfile) -> Result<String, AuthError> {
        // add 5 minutes to current unix epoch time as expiry date/time
        let exp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 24 * 3600 * 7;

    let claims = Claims {
        id: user_profile.id,
        name: user_profile.name.clone(),
        email: user_profile.email.clone(),
        exp: usize::try_from(exp).unwrap(),
    };

    // Create the authorization token
    let token = encode(&Header::default(), &claims, &KEYS.encoding)
        .map_err(|_| AuthError::TokenCreation)?;


    Ok(token)

}

// Returns the tokens if verified successfully
async fn verify_user(
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> Result<Redirect, AuthError> {

    let token_data = decode::<Claims>(&token, &KEYS.decoding, &Validation::default())
        .map_err(|_| AuthError::InvalidToken)?;

    let claim = token_data.claims;

    let mut tx = state.connection.begin().await.map_err(|_| AuthError::InternalServerError)?;

    let user_profile = match sqlx::query_as!(
        UserProfile,
        r#"
        UPDATE user_profile
        SET active = true
        WHERE email = $1
        RETURNING id, name, email, active, password as password_hash
        "#,
        claim.email
    )
    .fetch_one(&mut *tx)
    .await
    {
        Ok(profile) => profile,
        Err(e) => {
            println!("Error: {e}");
            return Err(AuthError::InternalServerError);
        }
    };

    let access_token = create_token(&user_profile).await?;

    tx.commit().await.map_err(|_| AuthError::InternalServerError)?;
    // to prevent update to commit in case error occurs.

    // frontend could use this access_token to directly send user to profile page
    Ok(Redirect::permanent(format!("http://0.0.0.0/3000/verified/{access_token}").as_str()))
}

async fn whoami(claims: Claims) -> Result<Json<Profile>, AuthError> {
    Ok(Json(Profile {
        id: claims.id,
        name: claims.name,
        email: claims.email,
    }))
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
        "SELECT id, name, email, active, password as password_hash FROM user_profile WHERE email = $1",
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

    if !user_profile.active {
        return Err(AuthError::UserNotActive);
    }

    // Create the authorization token
    let token = create_token(&user_profile).await?;

    // Send the authorized token
    Ok(Json(AuthBody::new(token)))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterAuthPayload>,
) -> Result<Json<ResponseBody>, AuthError> {
    if payload.email.is_empty() || payload.password.is_empty() || payload.name.is_empty() {
        return Err(AuthError::MissingCredentials);
    }

    let hashed_password = hash(payload.password, DEFAULT_COST).expect("Hashing Failed");

    let mut tx = state.connection.begin().await.map_err(|_| AuthError::InternalServerError)?;

    let user_profile = match sqlx::query_as!(
        UserProfile,
        r#"
        INSERT INTO user_profile (name, email, password, active)
        VALUES ($1, $2, $3, false)
        RETURNING id, name, email, active, password as password_hash
        "#,
        payload.name,
        payload.email,
        hashed_password
    )
    .fetch_one(&mut *tx)
    .await
    {
        Ok(profile) => profile,
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

    let token = create_token(&user_profile).await?;
    println!("token: {token}");
    // send email
    send_email(
        "Login to SAC",
        get_html(&token).as_str(),
        vec![user_profile.email.as_str()],
    )
    .await?;

    tx.commit().await.map_err(|_| AuthError::InternalServerError)?;

    // Send the authorized token
    Ok(Json(ResponseBody {
        message: String::from("Registered Successfully. Please verify the account to login."),
    }))
}
