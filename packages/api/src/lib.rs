use dioxus::fullstack::{WebSocketOptions, Websocket};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(feature = "server")]
use moka::future::Cache;

#[cfg(feature = "server")]
use tokio::sync::{Mutex, OnceCell};

#[cfg(feature = "server")]
use toasty::Db;

#[cfg(feature = "server")]
static DB: OnceCell<Mutex<Db>> = OnceCell::const_new();

#[cfg(feature = "server")]
static CACHE: OnceCell<Cache<u64, taste_db::Game>> = OnceCell::const_new();

#[cfg(feature = "server")]
async fn get_db() -> &'static Mutex<Db> {
    DB.get_or_init(|| async {
        let _ = dotenvy::dotenv();
        let _ = dotenvy::from_path(".env");
        let url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set (e.g. sqlite:tafl.db or postgres://...)");
        let db = toasty::Db::builder()
            .models(toasty::models!(taste_db::*))
            .connect(&url)
            .await
            .unwrap();
        Mutex::new(db)
    })
    .await
}

#[cfg(feature = "server")]
async fn get_cache() -> &'static Cache<u64, taste_db::Game> {
    CACHE.get_or_init(|| async {
        Cache::builder()
            .max_capacity(10_000)
            .time_to_live(std::time::Duration::from_secs(300))
            .build()
    })
    .await
}

#[cfg(feature = "server")]
#[get("/api/game/{game_id}")]
pub async fn get_game(game_id: u64) -> Result<taste_db::Game, ServerFnError> {
    let cache = get_cache().await;

    if let Some(game) = cache.get(&game_id).await {
        return Ok(game);
    }

    let db = get_db().await;
    let mut db = db.lock().await;
    let game = taste_db::Game::get_by_id(&mut *db, &game_id)
        .await
        .map_err(|e| ServerFnError::ServerError {
            message: e.to_string(),
            code: 0,
            details: None,
        })?;

    cache.insert(game_id, game.clone()).await;
    Ok(game)
}

#[cfg(feature = "server")]
#[post("/api/game")]
pub async fn create_game(
    variant: Option<String>,
    attacker_player: Option<String>,
    defender_player: Option<String>,
    attacker_elo: Option<u32>,
    defender_elo: Option<u32>,
    base_time_seconds: Option<u32>,
    increment_seconds: Option<u32>,
    is_private: Option<bool>,
) -> Result<taste_db::Game, ServerFnError> {
    let db = get_db().await;
    let mut db = db.lock().await;

    let game = toasty::create!(taste_db::Game {
        board: "3aaa3/4a4/4d4/a3d3a/aaddkddaa/a3d3a/4d4/4a4/3aaa3",
        attacker_player: attacker_player.unwrap_or_default(),
        defender_player: defender_player.unwrap_or_default(),
        attacker_elo: attacker_elo.unwrap_or(1500),
        defender_elo: defender_elo.unwrap_or(1500),
        current_side: "M",
        moves: "[]",
        result: "?",
        variant: variant.unwrap_or_else(|| "tablut".to_string()),
        base_time_seconds: base_time_seconds.unwrap_or(600),
        increment_seconds: increment_seconds.unwrap_or(5),
        is_private: is_private.unwrap_or(false),
        status: "active",
    })
    .exec(&mut *db)
    .await
    .map_err(|e| ServerFnError::ServerError {
        message: e.to_string(),
        code: 0,
        details: None,
    })?;

    let cache = get_cache().await;
    cache.insert(game.id, game.clone()).await;

    println!("Created game {} via API", game.id);
    Ok(game)
}

/// Echo the user input on the server.
#[post("/api/echo")]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    Ok(input)
}

#[post("/api/ping")]
pub async fn ping(input: u64) -> Result<String, ServerFnError> {
    let now = SystemTime::now();
    let target_time = UNIX_EPOCH + Duration::from_millis(input);
    let diff = target_time
        .duration_since(now)
        .unwrap_or_else(|e| e.duration());
    println!("The target timestamp is {} ms away.", diff.as_millis());

    Ok(diff.as_millis().to_string())
}

#[get("/api/ping_ws")]
pub async fn ping_ws(options: WebSocketOptions) -> Result<Websocket<u64, String>> {
    Ok(options.on_upgrade(move |mut socket| async move {
        while let Ok(msg) = socket.recv().await {
            let now = SystemTime::now();
            let target = UNIX_EPOCH + Duration::from_millis(msg);
            let diff = target.duration_since(now).unwrap_or_else(|e| e.duration());
            println!(
                "The WS target timestamp is {} micros away.",
                diff.as_micros()
            );
            _ = socket.send(diff.as_millis().to_string()).await;
        }
    }))
}
