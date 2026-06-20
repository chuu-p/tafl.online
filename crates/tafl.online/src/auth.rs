use axum::{
    extract::{Extension, Query},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use base64::Engine;
use diesel::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};

use tafl_domain::models::{NewUser, User};
use tafl_domain::schema::users;

pub use tafl_domain::establish_connection_to;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

pub const GOOGLE_CLIENT_ID: &str =
    "57900982813-4nasp99bt8g6n6g3n0en13v5fqu3qvvs.apps.googleusercontent.com";
pub const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

// ---------------------------------------------------------------------------
// Session store (in-memory)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Session {
    pub user_id: i32,
}

#[derive(Debug, Default)]
pub struct SessionStore {
    pub sessions: HashMap<String, Session>,
    pub pending_states: HashMap<String, String>, // state -> code_verifier
}

pub type SharedSession = Arc<RwLock<SessionStore>>;

// ---------------------------------------------------------------------------
// Shared auth state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AuthState {
    pub session_store: SharedSession,
    pub redirect_uri: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub db_url: String,
}

impl AuthState {
    pub fn new(redirect_uri: String) -> Self {
        let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "tafl.sqlite".to_string());
        Self {
            session_store: Arc::new(RwLock::new(SessionStore::default())),
            redirect_uri,
            token_url: GOOGLE_TOKEN_URL.to_string(),
            userinfo_url: GOOGLE_USERINFO_URL.to_string(),
            db_url,
        }
    }
}

// ---------------------------------------------------------------------------
// PKCE helpers
// ---------------------------------------------------------------------------

fn generate_pkce() -> (String, String) {
    let mut rng = rand::thread_rng();
    let verifier_bytes: [u8; 32] = rng.gen();
    let code_verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(verifier_bytes);

    let digest = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);

    (code_verifier, code_challenge)
}

fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

fn generate_session_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    pub code: String,
    pub state: String,
    #[serde(default)]
    pub error: Option<String>,
}

pub async fn login_handler(Extension(state): Extension<AuthState>) -> Response {
    let (code_verifier, code_challenge) = generate_pkce();
    let state_param = generate_state();

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        GOOGLE_AUTH_URL,
        GOOGLE_CLIENT_ID,
        urlencoding::encode(&state.redirect_uri),
        urlencoding::encode("openid email profile"),
        state_param,
        code_challenge,
    );

    {
        let mut store = state.session_store.write().await;
        store.pending_states.insert(state_param, code_verifier);
    }

    Redirect::temporary(&auth_url).into_response()
}

pub async fn callback_handler(
    Extension(state): Extension<AuthState>,
    cookies: Cookies,
    Query(params): Query<CallbackParams>,
) -> Result<Redirect, (StatusCode, String)> {
    if let Some(error) = &params.error {
        return Err((StatusCode::BAD_REQUEST, format!("OAuth error: {error}")));
    }

    let code_verifier = {
        let mut store = state.session_store.write().await;
        store.pending_states.remove(&params.state).ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid or expired state".to_string(),
            )
        })?
    };

    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();

    let http = reqwest::Client::new();

    let token_response: TokenResponse = {
        let resp = http
            .post(&state.token_url)
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", GOOGLE_CLIENT_ID),
                ("client_secret", &client_secret),
                ("code", &params.code),
                ("redirect_uri", &state.redirect_uri),
                ("code_verifier", &code_verifier),
            ])
            .send()
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Token request failed: {e}"),
                )
            })?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read token response body: {e}"),
            )
        })?;

        if !status.is_success() {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Token exchange failed ({}): {}", status, body),
            ));
        }

        serde_json::from_str(&body).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Token parse failed: {e}. Body: {body}"),
            )
        })?
    };

    let user_info: GoogleUserInfo = http
        .get(&state.userinfo_url)
        .bearer_auth(&token_response.access_token)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Userinfo request failed: {e}"),
            )
        })?
        .json()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Userinfo parse failed: {e}"),
            )
        })?;

    let user = upsert_user(&state.db_url, &user_info)?;

    let session_token = generate_session_token();
    {
        let mut store = state.session_store.write().await;
        store
            .sessions
            .insert(session_token.clone(), Session { user_id: user.id });
    }

    let mut cookie = Cookie::new("session_token", session_token);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
    cookies.add(cookie);

    Ok(Redirect::temporary("/"))
}

pub async fn me_handler(
    Extension(state): Extension<AuthState>,
    cookies: Cookies,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let session_token = cookies
        .get("session_token")
        .map(|c| c.value().to_string())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user_id = {
        let store = state.session_store.read().await;
        store
            .sessions
            .get(&session_token)
            .map(|s| s.user_id)
            .ok_or(StatusCode::UNAUTHORIZED)?
    };

    let mut conn = establish_connection_to(&state.db_url);
    let user: User = users::table
        .find(user_id)
        .first(&mut conn)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(axum::Json(serde_json::to_value(user).unwrap()))
}

pub async fn logout_handler(Extension(state): Extension<AuthState>, cookies: Cookies) -> Response {
    if let Some(session_cookie) = cookies.get("session_token") {
        let token = session_cookie.value().to_string();
        let mut store = state.session_store.write().await;
        store.sessions.remove(&token);
    }

    let mut cookie = Cookie::new("session_token", "");
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_max_age(tower_cookies::cookie::time::Duration::ZERO);
    cookies.add(cookie);

    Redirect::temporary("/").into_response()
}

// ---------------------------------------------------------------------------
// Google API types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    #[serde(default)]
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub id_token: Option<String>,
}

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

fn upsert_user(db_url: &str, info: &GoogleUserInfo) -> Result<User, (StatusCode, String)> {
    let mut conn = establish_connection_to(db_url);

    let existing: Option<User> = users::table
        .filter(users::google_id.eq(&info.id))
        .first(&mut conn)
        .optional()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("DB query failed: {e}"),
            )
        })?;

    if let Some(user) = existing {
        diesel::update(users::table.filter(users::id.eq(user.id)))
            .set((
                users::email.eq(&info.email),
                users::name.eq(&info.name),
                users::avatar_url.eq(&info.picture),
                users::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("DB update failed: {e}"),
                )
            })?;

        users::table.find(user.id).first(&mut conn).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("DB re-read failed: {e}"),
            )
        })
    } else {
        let new_user = NewUser {
            google_id: &info.id,
            email: &info.email,
            name: &info.name,
            avatar_url: info.picture.as_deref(),
        };

        diesel::insert_into(users::table)
            .values(&new_user)
            .execute(&mut conn)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("DB insert failed: {e}"),
                )
            })?;

        users::table
            .order(users::id.desc())
            .first(&mut conn)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("DB re-read failed: {e}"),
                )
            })
    }
}

// ---------------------------------------------------------------------------
// Router — returns Router<()> ready to merge, with CookieManagerLayer
// ---------------------------------------------------------------------------

pub fn auth_router(state: AuthState) -> Router {
    Router::new()
        .route("/api/auth/login", get(login_handler))
        .route("/api/auth/callback", get(callback_handler))
        .route("/api/auth/me", get(me_handler))
        .route("/api/auth/logout", get(logout_handler))
        .layer(CookieManagerLayer::new())
        .layer(Extension(state))
}
