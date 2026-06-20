use serde_json::{json, Value};

async fn post_json(url: &str, body: Value) -> Value {
    let client = reqwest::Client::new();
    let resp = client.post(url).json(&body).send().await.unwrap();
    assert!(resp.status().is_success(), "POST {} failed: {}", url, resp.status());
    resp.json().await.unwrap()
}

async fn get_json(url: &str) -> Value {
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await.unwrap();
    assert!(resp.status().is_success(), "GET {} failed: {}", url, resp.status());
    resp.json().await.unwrap()
}

#[tokio::test]
async fn e2e_full_game_flow() {
    // Start the app server
    let _ = dotenvy::dotenv();
    let auth_state = tafl_online::auth::AuthState::new("http://localhost:8080/api/auth/callback".to_string());
    let game_state = tafl_online::game::new_state();

    let app = axum::Router::new()
        .route("/api/game/create", axum::routing::post(tafl_online::game::create_handler))
        .route("/api/game/list", axum::routing::get(tafl_online::game::list_handler))
        .route("/api/game/{id}/join", axum::routing::post(tafl_online::game::join_handler))
        .route("/api/game/{id}/move", axum::routing::post(tafl_online::game::move_handler))
        .route("/api/game/{id}/state", axum::routing::get(tafl_online::game::state_handler))
        .route("/api/game/{id}/messages", axum::routing::get(tafl_online::game::messages_handler))
        .route("/api/game/{id}/resign", axum::routing::post(tafl_online::game::resign_handler))
        .layer(axum::extract::Extension(game_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let base = format!("http://{}", addr);

    // 1. Create game
    let created = post_json(&format!("{}/api/game/create", base), json!({
        "variant": "tablut",
        "minutes": 5,
        "increment": 3
    })).await;
    let room_id = created["room_id"].as_str().unwrap().to_string();
    println!("Created game: {}", room_id);

    // 2. List games — should show our game
    let games = get_json(&format!("{}/api/game/list", base)).await;
    assert!(games.as_array().unwrap().iter().any(|g| g["room_id"] == room_id));

    // 3. Join as Swedes (white)
    let join1 = post_json(&format!("{}/api/game/{}/join", base, room_id), json!({
        "name": "Alice"
    })).await;
    assert_eq!(join1["side"], "Swedes");
    println!("Alice joined as Swedes");

    // 4. Join as Muscovites (black)
    let join2 = post_json(&format!("{}/api/game/{}/join", base, room_id), json!({
        "name": "Bob"
    })).await;
    assert_eq!(join2["side"], "Muscovites");
    println!("Bob joined as Muscovites");

    // 5. Check state — game should be started
    let state = get_json(&format!("{}/api/game/{}/state", base, room_id)).await;
    assert_eq!(state["turn"], "Swedes");
    println!("Game started, turn: Swedes");

    // 6. Swedes move: defender at (3,4) to (3,3)
    let move1 = post_json(&format!("{}/api/game/{}/move", base, room_id), json!({
        "side": "Swedes",
        "from": [3, 4],
        "to": [3, 3]
    })).await;
    assert_eq!(move1["ok"], true);
    println!("Swedes moved (3,4) -> (3,3)");

    // 7. Check state — turn should be Muscovites
    let state = get_json(&format!("{}/api/game/{}/state", base, room_id)).await;
    assert_eq!(state["turn"], "Muscovites");
    println!("Turn changed to Muscovites");

    // 8. Muscovites move: attacker at (1,4) to (1,3)
    let move2 = post_json(&format!("{}/api/game/{}/move", base, room_id), json!({
        "side": "Muscovites",
        "from": [1, 4],
        "to": [1, 3]
    })).await;
    assert_eq!(move2["ok"], true);
    println!("Muscovites moved (1,4) -> (1,3)");

    // 9. Check state — turn should be back to Swedes
    let state = get_json(&format!("{}/api/game/{}/state", base, room_id)).await;
    assert_eq!(state["turn"], "Swedes");
    println!("Turn back to Swedes");

    // 10. Swedes make another move: defender at (4,3) to (3,3)... wait, (3,3) is now occupied
    // Move defender at (5,4) to (5,3)
    let move3 = post_json(&format!("{}/api/game/{}/move", base, room_id), json!({
        "side": "Swedes",
        "from": [5, 4],
        "to": [5, 3]
    })).await;
    assert_eq!(move3["ok"], true);
    println!("Swedes moved (5,4) -> (5,3)");

    // 11. Check state
    let state = get_json(&format!("{}/api/game/{}/state", base, room_id)).await;
    assert_eq!(state["turn"], "Muscovites");
    println!("Turn back to Muscovites");

    // 12. Check timers — white should have gained increment
    let white_time = state["white_time"].as_u64().unwrap();
    let black_time = state["black_time"].as_u64().unwrap();
    println!("Timers: white={}s, black={}s", white_time, black_time);
    // White started at 300, moved once (+3 increment) = 303
    assert!(white_time >= 300, "White time should be >= 300, got {}", white_time);

    // 13. Check messages
    let messages = get_json(&format!("{}/api/game/{}/messages", base, room_id)).await;
    let msgs = messages.as_array().unwrap();
    println!("Messages: {}", msgs.len());
    assert!(msgs.len() >= 3, "Should have at least join+join+start+move messages");

    // 14. Muscovites resign
    let resign = post_json(&format!("{}/api/game/{}/resign", base, room_id), json!({
        "side": "Muscovites"
    })).await;
    assert_eq!(resign["ok"], true);
    println!("Bob resigned");

    // 15. Check final state — Swedes should win
    let state = get_json(&format!("{}/api/game/{}/state", base, room_id)).await;
    assert_eq!(state["winner"], "Swedes");
    println!("Game over! Winner: Swedes (Alice)");

    // 16. Try to move after game over — should fail
    let move_after = client_post(&format!("{}/api/game/{}/move", base, room_id), json!({
        "side": "Swedes",
        "from": [2, 4],
        "to": [3, 4]
    })).await;
    assert!(!move_after.status().is_success(), "Move after game over should fail");

    println!("\n=== E2E test passed! ===");
}

async fn client_post(url: &str, body: Value) -> reqwest::Response {
    reqwest::Client::new().post(url).json(&body).send().await.unwrap()
}
