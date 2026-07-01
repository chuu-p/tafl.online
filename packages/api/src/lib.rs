//! This crate contains all shared fullstack server functions.
use dioxus::fullstack::{WebSocketOptions, Websocket};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(feature = "server")]
use moka::future::Cache;

#[cfg(feature = "server")]
use tokio::sync::OnceCell;

#[cfg(feature = "server")]
static CACHE: OnceCell<Cache<String, tafl_game::Game>> = OnceCell::const_new();

#[cfg(feature = "server")]
static DB: OnceCell<Db> = OnceCell::const_new();

#[cfg(feature = "server")]
async fn get_cache() -> &'static Cache<String, tafl_game::Game> {
    CACHE.get_or_init(|| async {
        Cache::builder()
            .max_capacity(10_000)
            .time_to_live(std::time::Duration::from_secs(300))
    })
}

#[cfg(feature = "server")]
async fn get_db() -> &'static Db {
    DB.get_or_init(|| async {
        let _ = dotenvy::dotenv();
        let _ = dotenvy::from_path(".env");
        let url = std::env::var("DATABASE_URL").map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "DATABASE_URL must be set (e.g. sqlite:tafl.db or postgres://...)",
            )
        })?;
        toasty::Db::builder()
            .models(toasty::models!(crate::*))
            .connect(url)
            .await
            .unwrap()
    })
}

#[get("/api/game/:id")]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    Ok(input)
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


#[get("/api/game/{game_id}")]
async fn get_game(game_id: Uuid) -> Result<taste_db::Game> {
    let mut db = get_db().await;
    let mut cache = get_cache().await.clone();
    
    if let Some(game) = cache.get(&game_id).await {
        return game;
    }

    // cache miss, check db
    if let Some(game) = taste_db::Game::find()
        .select(Game::fields().id)
        .exec(&mut db)
        .await? {
            // cache this entry before returning
            cache.insert(game_id, game).await;
            return game;
    }

    None
}

