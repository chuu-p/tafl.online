use diesel::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use tafl_online::auth::{auth_router, AuthState};

fn setup_test_db(name: &str) -> String {
    let db_url = format!("test_auth_{}.sqlite", name);
    let _ = std::fs::remove_file(&db_url);

    let mut conn = tafl_domain::establish_connection_to(&db_url);
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            google_id TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL,
            name TEXT NOT NULL,
            avatar_url TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&mut conn)
    .expect("Failed to create test table");

    db_url
}

fn cleanup_test_db(db_url: &str) {
    let _ = std::fs::remove_file(db_url);
    let _ = std::fs::remove_file(format!("{db_url}-wal"));
    let _ = std::fs::remove_file(format!("{db_url}-shm"));
}

fn test_state(mock_server_uri: &str, db_url: &str) -> AuthState {
    AuthState {
        session_store: Arc::new(RwLock::new(Default::default())),
        redirect_uri: "http://localhost:3000/api/auth/callback".to_string(),
        token_url: format!("{}/token", mock_server_uri),
        userinfo_url: format!("{}/userinfo", mock_server_uri),
        db_url: db_url.to_string(),
    }
}

#[tokio::test]
async fn test_full_auth_flow() {
    let db_url = setup_test_db("full_flow");

    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "fake_access_token_123",
            "token_type": "Bearer",
            "expires_in": 3600,
            "id_token": "fake_id_token"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/userinfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "google_user_123",
            "email": "test@example.com",
            "name": "Test User",
            "picture": "https://example.com/photo.jpg"
        })))
        .mount(&mock_server)
        .await;

    let state = test_state(&mock_server.uri(), &db_url);
    let app = auth_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    // Step 1: Login -> redirect to Google
    let login_response = client
        .get(format!("http://{}/api/auth/login", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(login_response.status(), 307);

    let location = login_response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    assert!(location.contains("client_id="));
    assert!(location.contains("response_type=code"));
    assert!(location.contains("scope="));
    assert!(location.contains("state="));
    assert!(location.contains("code_challenge="));
    assert!(location.contains("code_challenge_method=S256"));

    let url = url::Url::parse(&location).unwrap();
    let state_param: String = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string())
        .expect("state parameter not found");

    // Step 2: Callback with valid state + code
    let callback_response = client
        .get(format!(
            "http://{}/api/auth/callback?code=test_auth_code&state={}",
            addr, state_param
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(callback_response.status(), 307);
    assert_eq!(
        callback_response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap(),
        "/"
    );

    let cookies = callback_response.cookies().collect::<Vec<_>>();
    let session_cookie = cookies.iter().find(|c| c.name() == "session_token");
    assert!(session_cookie.is_some(), "Session cookie should be set");

    let session_token = session_cookie.unwrap().value().to_string();

    // Step 3: /api/auth/me with session cookie -> user info
    let me_response = client
        .get(format!("http://{}/api/auth/me", addr))
        .header("Cookie", format!("session_token={}", session_token))
        .send()
        .await
        .unwrap();

    assert_eq!(me_response.status(), 200);

    let user: Value = me_response.json().await.unwrap();
    assert_eq!(user["google_id"], "google_user_123");
    assert_eq!(user["email"], "test@example.com");
    assert_eq!(user["name"], "Test User");
    assert_eq!(user["avatar_url"], "https://example.com/photo.jpg");

    // Step 4: Logout
    let logout_response = client
        .get(format!("http://{}/api/auth/logout", addr))
        .header("Cookie", format!("session_token={}", session_token))
        .send()
        .await
        .unwrap();

    assert_eq!(logout_response.status(), 307);

    // Step 5: Verify session is gone after logout
    let me_after_logout = client
        .get(format!("http://{}/api/auth/me", addr))
        .header("Cookie", format!("session_token={}", session_token))
        .send()
        .await
        .unwrap();

    assert_eq!(me_after_logout.status(), 401);

    cleanup_test_db(&db_url);
}

#[tokio::test]
async fn test_callback_with_invalid_state() {
    let db_url = setup_test_db("invalid_state");

    let mock_server = MockServer::start().await;
    let state = test_state(&mock_server.uri(), &db_url);
    let app = auth_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let response = client
        .get(format!(
            "http://{}/api/auth/callback?code=test&state=invalid_state",
            addr
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
    let body = response.text().await.unwrap();
    assert!(body.contains("Invalid or expired state"));

    cleanup_test_db(&db_url);
}

#[tokio::test]
async fn test_me_without_session() {
    let db_url = setup_test_db("no_session");

    let mock_server = MockServer::start().await;
    let state = test_state(&mock_server.uri(), &db_url);
    let app = auth_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let response = client
        .get(format!("http://{}/api/auth/me", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 401);

    cleanup_test_db(&db_url);
}
