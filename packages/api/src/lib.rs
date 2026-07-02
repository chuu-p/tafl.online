use dioxus::fullstack::{WebSocketOptions, Websocket};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "ping")]
    Ping { client_time_ms: u64 },
    #[serde(rename = "subscribe_game")]
    SubscribeGame { game_id: u64 },
    #[serde(rename = "move")]
    MakeMove { from_sq: u8, to_sq: u8 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "pong")]
    Pong { client_time_ms: u64, server_time_ms: u64 },
    #[serde(rename = "game_state")]
    GameState {
        game_id: u64,
        board: String,
        current_side: String,
        moves: String,
        result: String,
    },
    #[serde(rename = "move_result")]
    MoveResult {
        accepted: bool,
        from_sq: u8,
        to_sq: u8,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

#[cfg(feature = "server")]
use moka::future::Cache;

#[cfg(feature = "server")]
use std::collections::HashMap;

#[cfg(feature = "server")]
use tokio::sync::{Mutex, OnceCell};

#[cfg(feature = "server")]
use toasty::Db;

#[cfg(feature = "server")]
static DB: OnceCell<Mutex<Db>> = OnceCell::const_new();

#[cfg(feature = "server")]
static CACHE: OnceCell<Cache<u64, taste_db::Game>> = OnceCell::const_new();

#[cfg(feature = "server")]
static GAME_ROOMS: OnceCell<Mutex<HashMap<u64, tokio::sync::broadcast::Sender<ServerMessage>>>> =
    OnceCell::const_new();

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
    CACHE
        .get_or_init(|| async {
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

#[get("/api/game/{game_id}/ws")]
pub async fn get_game_ws(
    game_id: u64,
    options: WebSocketOptions,
) -> Result<Websocket<ClientMessage, ServerMessage>> {
    Ok(options.on_upgrade(move |socket| async move {
        use futures::{SinkExt, StreamExt};

        let (mut sender, mut receiver) = socket.split();
        let tx = get_game_room(game_id).await;
        let mut rx = tx.subscribe();

        if let Ok(Some(state)) = load_game_state_msg(game_id).await {
            sender.send(state).await.ok();
        }

        loop {
            tokio::select! {
                msg = receiver.next() => {
                    match msg {
                        Some(Ok(ClientMessage::Ping { client_time_ms })) => {
                            let server_time_ms = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            sender.send(ServerMessage::Pong { client_time_ms, server_time_ms }).await.ok();
                        }
                        Some(Ok(ClientMessage::SubscribeGame { game_id: new_id })) => {
                            if let Ok(Some(state)) = load_game_state_msg(new_id).await {
                                sender.send(state).await.ok();
                            }
                        }
                        Some(Ok(ClientMessage::MakeMove { from_sq, to_sq })) => {
                            match apply_move(game_id, from_sq, to_sq).await {
                                Ok(updated_state) => {
                                    tx.send(updated_state).ok();
                                }
                                Err(_e) => {
                                    sender.send(ServerMessage::MoveResult { accepted: false, from_sq, to_sq }).await.ok();
                                }
                            }
                        }
                        Some(Err(_)) | None => break,
                    }
                }
                update = rx.recv() => {
                    match update {
                        Ok(msg) => { sender.send(msg).await.ok(); }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
            }
        }
    }))
}

#[cfg(feature = "server")]
async fn get_game_room(game_id: u64) -> tokio::sync::broadcast::Sender<ServerMessage> {
    let rooms = GAME_ROOMS
        .get_or_init(|| async { Mutex::new(HashMap::new()) })
        .await;
    let mut rooms = rooms.lock().await;
    rooms
        .entry(game_id)
        .or_insert_with(|| {
            let (tx, _) = tokio::sync::broadcast::channel(32);
            tx
        })
        .clone()
}

#[cfg(feature = "server")]
async fn load_game_state_msg(game_id: u64) -> Result<Option<ServerMessage>, ServerFnError> {
    let db = get_db().await;
    let mut db = db.lock().await;
    let db_game = taste_db::Game::get_by_id(&mut *db, &game_id).await.map_err(|e| ServerFnError::ServerError {
            message: e.to_string(), code: 0, details: None,
        })?;
    Ok(Some(ServerMessage::GameState {
        game_id,
        board: db_game.board,
        current_side: db_game.current_side,
        moves: db_game.moves,
        result: db_game.result,
    }))
}

#[cfg(feature = "server")]
async fn apply_move(game_id: u64, from_sq: u8, to_sq: u8) -> Result<ServerMessage, ServerFnError> {
    let cache = get_cache().await;
    let db = get_db().await;
    let mut db = db.lock().await;

    let mut db_game = if let Some(cached) = cache.get(&game_id).await {
        cached
    } else {
        taste_db::Game::get_by_id(&mut *db, &game_id)
            .await
            .map_err(|e| ServerFnError::ServerError {
                message: e.to_string(), code: 0, details: None,
            })?
    };

    let game: tafl_game::Game = db_game.clone().into();
    let from = tafl_game::Pos::from_index(from_sq).ok_or_else(|| ServerFnError::ServerError {
            message: "invalid from".into(), code: 0, details: None,
        })?;
    let to = tafl_game::Pos::from_index(to_sq).ok_or_else(|| ServerFnError::ServerError {
            message: "invalid to".into(), code: 0, details: None,
        })?;

    let new_game = game.make_move(from, to).ok_or_else(|| ServerFnError::ServerError {
            message: "move rejected".into(), code: 0, details: None,
        })?;

    let db_new: taste_db::Game = new_game.into();
    toasty::update!(db_game {
        board: db_new.board.as_str(),
        moves: db_new.moves.as_str(),
        current_side: db_new.current_side.as_str(),
        result: db_new.result.as_str(),
    })
    .exec(&mut *db)
    .await
    .map_err(|e| ServerFnError::ServerError {
        message: e.to_string(), code: 0, details: None,
    })?;

    cache.insert(game_id, db_new.clone()).await;

    println!("Move {from_sq}->{to_sq} applied to game {game_id}");

    Ok(ServerMessage::GameState {
        game_id,
        board: db_new.board,
        current_side: db_new.current_side,
        moves: db_new.moves,
        result: db_new.result,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_roundtrip() {
        let ping = ClientMessage::Ping {
            client_time_ms: 12345,
        };
        let json = serde_json::to_string(&ping).unwrap();
        assert_eq!(json, r#"{"type":"ping","client_time_ms":12345}"#);
        let de: ClientMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(de, ClientMessage::Ping { client_time_ms: 12345 }));

        let sub = ClientMessage::SubscribeGame { game_id: 99 };
        let json = serde_json::to_string(&sub).unwrap();
        assert_eq!(json, r#"{"type":"subscribe_game","game_id":99}"#);

        let mv = ClientMessage::MakeMove { from_sq: 10, to_sq: 20 };
        let json = serde_json::to_string(&mv).unwrap();
        assert_eq!(json, r#"{"type":"move","from_sq":10,"to_sq":20}"#);

        let pong = ServerMessage::Pong {
            client_time_ms: 12345,
            server_time_ms: 67890,
        };
        let json = serde_json::to_string(&pong).unwrap();
        assert_eq!(
            json,
            r#"{"type":"pong","client_time_ms":12345,"server_time_ms":67890}"#
        );

        let gs = ServerMessage::GameState {
            game_id: 1,
            board: "3aaa3/...".into(),
            current_side: "M".into(),
            moves: "[]".into(),
            result: "?".into(),
        };
        let json = serde_json::to_string(&gs).unwrap();
        assert_eq!(
            json,
            r#"{"type":"game_state","game_id":1,"board":"3aaa3/...","current_side":"M","moves":"[]","result":"?"}"#
        );

        let mr = ServerMessage::MoveResult {
            accepted: true,
            from_sq: 10,
            to_sq: 20,
        };
        let json = serde_json::to_string(&mr).unwrap();
        assert_eq!(
            json,
            r#"{"type":"move_result","accepted":true,"from_sq":10,"to_sq":20}"#
        );
    }

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn test_game_room_broadcast() {
        let tx = get_game_room(42).await;
        let mut rx1 = tx.subscribe();
        let mut rx2 = tx.subscribe();

        tx.send(ServerMessage::Pong {
            client_time_ms: 1,
            server_time_ms: 2,
        })
        .unwrap();

        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();
        assert!(matches!(msg1, ServerMessage::Pong { .. }));
        assert!(matches!(msg2, ServerMessage::Pong { .. }));
    }

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn test_game_move_apply() {
        use toasty::Db;
        let mut db = Db::builder()
            .models(toasty::models!(taste_db::*))
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db.push_schema().await.unwrap();

        let initial_board = tafl_game::Game::new()
            .ten()
            .split(' ')
            .next()
            .unwrap()
            .to_string();

        let mut db_game = toasty::create!(taste_db::Game {
            board: initial_board.as_str(),
            attacker_player: "",
            defender_player: "",
            attacker_elo: 1500,
            defender_elo: 1500,
            current_side: "M",
            moves: "[]",
            result: "?",
            variant: "tablut",
            base_time_seconds: 600,
            increment_seconds: 5,
            is_private: false,
            status: "active",
        })
        .exec(&mut db)
        .await
        .unwrap();

        let game_id = db_game.id;

        // A4(3) -> B4(12): valid attacker move (down one square)
        let game: tafl_game::Game = db_game.clone().into();
        let from = tafl_game::Pos::from_index(3).unwrap();
        let to = tafl_game::Pos::from_index(12).unwrap();
        let new_game = game.make_move(from, to).expect("valid move");

        assert_eq!(new_game.moves.len(), 1);
        assert_eq!(new_game.current_side, tafl_game::Side::Defender);

        // Convert back to DB and persist
        let db_updated: taste_db::Game = new_game.into();
        toasty::update!(db_game {
            board: db_updated.board.as_str(),
            moves: db_updated.moves.as_str(),
            current_side: db_updated.current_side.as_str(),
            result: db_updated.result.as_str(),
        })
        .exec(&mut db)
        .await
        .unwrap();

        // Verify persisted state
        let reloaded = taste_db::Game::get_by_id(&mut db, &game_id).await.unwrap();
        assert_eq!(reloaded.current_side, "S");
        assert!(reloaded.moves.len() > 5); // not empty JSON
    }

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn test_broadcast_via_shared_room() {
        let tx = get_game_room(1).await;
        let mut rx_a = tx.subscribe();
        let mut rx_b = tx.subscribe();

        // Simulate a move broadcast from one client reaching all others
        tx.send(ServerMessage::MoveResult {
            accepted: true,
            from_sq: 36,
            to_sq: 27,
        })
        .unwrap();

        let a = rx_a.recv().await.unwrap();
        let b = rx_b.recv().await.unwrap();
        assert_eq!(a, b);
        assert!(matches!(a, ServerMessage::MoveResult { accepted: true, .. }));
    }
}
