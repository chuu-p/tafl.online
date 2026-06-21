use reqwest::Client;
use serde_json::{json, Value};

async fn post_json(client: &Client, url: &str, body: Value) -> Value {
    let resp = client.post(url).json(&body).send().await.unwrap();
    assert!(resp.status().is_success(), "POST {} failed: {} {}", url, resp.status(), resp.text().await.unwrap_or_default());
    resp.json().await.unwrap()
}

async fn get_json(client: &Client, url: &str) -> Value {
    let resp = client.get(url).send().await.unwrap();
    assert!(resp.status().is_success(), "GET {} failed: {}", url, resp.status());
    resp.json().await.unwrap()
}

#[tokio::test]
async fn e2e_two_players() {
    let _ = dotenvy::dotenv();
    let game_state = tafl_online::game::new_state();
    let auth_state = tafl_online::auth::AuthState::new("http://localhost:8080/api/auth/callback".to_string());

    let app = axum::Router::new()
        .route("/api/game/create", axum::routing::post(tafl_online::game::create_handler))
        .route("/api/game/list", axum::routing::get(tafl_online::game::list_handler))
        .route("/api/game/{id}/join", axum::routing::post(tafl_online::game::join_handler))
        .route("/api/game/{id}/move", axum::routing::post(tafl_online::game::move_handler))
        .route("/api/game/{id}/state", axum::routing::get(tafl_online::game::state_handler))
        .route("/api/game/{id}/messages", axum::routing::get(tafl_online::game::messages_handler))
        .route("/api/game/{id}/resign", axum::routing::post(tafl_online::game::resign_handler))
        .layer(tower_cookies::CookieManagerLayer::new())
        .layer(axum::extract::Extension(auth_state.clone()))
        .layer(axum::extract::Extension(game_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    let base = format!("http://{}", addr);

    // Create two "browsers" (HTTP clients with cookie jars)
    let browser1 = Client::builder().cookie_store(true).build().unwrap();
    let browser2 = Client::builder().cookie_store(true).build().unwrap();

    // Both users log in (simulate by creating sessions directly)
    // In real flow, they'd go through OAuth, but for testing we create sessions directly
    let user1_id = create_test_user(&auth_state, "alice@example.com", "Alice").await;
    let user2_id = create_test_user(&auth_state, "bob@example.com", "Bob").await;
    println!("User 1 (Alice) id: {}", user1_id);
    println!("User 2 (Bob) id: {}", user2_id);

    // Set session cookies for both browsers via header on every request
    let session1 = login_as(&auth_state, user1_id).await;
    let session2 = login_as(&auth_state, user2_id).await;
    println!("Session 1: {}", session1);
    println!("Session 2: {}", session2);

    // Alice creates a game
    let created = browser1.post(&format!("{}/api/game/create", base))
        .header("Cookie", format!("session_token={}", session1))
        .json(&json!({"variant": "tablut", "minutes": 5, "increment": 3}))
        .send().await.unwrap().json::<Value>().await.unwrap();
    let room_id = created["room_id"].as_str().unwrap().to_string();
    println!("Alice created game: {}", room_id);

    // Alice joins as Swedes
    let j1 = browser1.post(&format!("{}/api/game/{}/join", base, room_id))
        .header("Cookie", format!("session_token={}", session1))
        .json(&json!({}))
        .send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(j1["side"], "Swedes");
    println!("Alice joined as Swedes");

    // Bob joins as Muscovites
    let j2 = browser2.post(&format!("{}/api/game/{}/join", base, room_id))
        .header("Cookie", format!("session_token={}", session2))
        .json(&json!({}))
        .send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(j2["side"], "Muscovites");
    println!("Bob joined as Muscovites");

    // Check initial state
    let state = browser1.get(&format!("{}/api/game/{}/state", base, room_id))
        .header("Cookie", format!("session_token={}", session1))
        .send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(state["turn"], "Swedes");
    println!("Game started, turn: Swedes");

    // Alice (Swedes) makes a move
    let m1 = browser1.post(&format!("{}/api/game/{}/move", base, room_id))
        .header("Cookie", format!("session_token={}", session1))
        .json(&json!({"side": "Swedes", "from": [3, 4], "to": [3, 3]}))
        .send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(m1["ok"], true);
    println!("Alice moved (3,4) -> (3,3)");

    // Bob (Muscovites) makes a move
    let m2 = browser2.post(&format!("{}/api/game/{}/move", base, room_id))
        .header("Cookie", format!("session_token={}", session2))
        .json(&json!({"side": "Muscovites", "from": [1, 4], "to": [1, 3]}))
        .send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(m2["ok"], true);
    println!("Bob moved (1,4) -> (1,3)");

    // Alice makes another move
    let m3 = browser1.post(&format!("{}/api/game/{}/move", base, room_id))
        .header("Cookie", format!("session_token={}", session1))
        .json(&json!({"side": "Swedes", "from": [5, 4], "to": [5, 3]}))
        .send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(m3["ok"], true);
    println!("Alice moved (5,4) -> (5,3)");

    // Check state
    let state = browser1.get(&format!("{}/api/game/{}/state", base, room_id))
        .header("Cookie", format!("session_token={}", session1))
        .send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(state["turn"], "Muscovites");
    println!("Turn: Muscovites");

    // Bob resigns
    let resign = browser2.post(&format!("{}/api/game/{}/resign", base, room_id))
        .header("Cookie", format!("session_token={}", session2))
        .json(&json!({"side": "Muscovites"}))
        .send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(resign["ok"], true);
    println!("Bob resigned");

    // Check final state
    let state = browser1.get(&format!("{}/api/game/{}/state", base, room_id))
        .header("Cookie", format!("session_token={}", session1))
        .send().await.unwrap().json::<Value>().await.unwrap();
    assert_eq!(state["winner"], "Swedes");
    println!("Winner: Swedes (Alice)");

    println!("\n=== Two-player test passed! ===");
}

async fn create_test_user(auth_state: &tafl_online::auth::AuthState, email: &str, name: &str) -> i32 {
    use diesel::prelude::*;
    use tafl_domain::models::User;
    use tafl_domain::models::NewUser;
    use tafl_domain::schema::users;

    let mut conn = tafl_domain::establish_connection_to(&auth_state.db_url);

    // Create table if not exists, clear old data
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
    .unwrap();
    diesel::sql_query("DELETE FROM users").execute(&mut conn).unwrap();

    let new_user = NewUser {
        google_id: &format!("test_{}", email),
        email,
        name,
        avatar_url: None,
    };
    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&mut conn)
        .unwrap();
    users::table.order(users::id.desc()).first::<User>(&mut conn).unwrap().id
}

async fn login_as(auth_state: &tafl_online::auth::AuthState, user_id: i32) -> String {
    use std::collections::HashMap;
    let token = format!("test_token_{}", user_id);
    let mut store = auth_state.session_store.write().await;
    store.sessions.insert(token.clone(), tafl_online::auth::Session { user_id });
    token
}
