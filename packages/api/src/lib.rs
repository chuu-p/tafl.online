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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub id: u64,
    pub board: String,
    pub current_side: String,
    pub moves: String,
    pub result: String,
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
static ACTIVE_GAMES: OnceCell<dashmap::DashMap<u64, tafl_game::Game>> = OnceCell::const_new();

#[cfg(feature = "server")]
pub static PENDING_DB_WRITES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(feature = "server")]
static TRACING_INITED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "server")]
fn init_tracing() {
    if TRACING_INITED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }

    use opentelemetry::global;
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::trace::TracerProvider;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://127.0.0.1:4317")
        .build()
        .unwrap();

    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::TokioCurrentThread)
        .build();

    let tracer = provider.tracer("tafl-online");
    global::set_tracer_provider(provider);

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .with(telemetry)
        .try_init();
}

#[cfg(feature = "server")]
pub async fn wait_for_pending_writes() {
    use std::sync::atomic::Ordering;
    while PENDING_DB_WRITES.load(Ordering::SeqCst) > 0 {
        tokio::task::yield_now().await;
    }
}

#[cfg(feature = "server")]
#[tracing::instrument]
async fn get_db() -> &'static Mutex<Db> {
    DB.get_or_init(|| async {
        init_tracing();
        let _ = dotenvy::dotenv();
        let _ = dotenvy::from_path(".env");
        let url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set (e.g. sqlite:tafl.db or postgres://...)");
        let db = toasty::Db::builder()
            .models(toasty::models!(taste_db::*))
            .connect(&url)
            .await
            .unwrap();
        db.push_schema().await.ok();
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
async fn get_active_games() -> &'static dashmap::DashMap<u64, tafl_game::Game> {
    ACTIVE_GAMES
        .get_or_init(|| async { dashmap::DashMap::new() })
        .await
}

#[cfg(feature = "server")]
fn game_to_state_msg(game_id: u64, g: &tafl_game::Game) -> ServerMessage {
    let db: taste_db::Game = g.clone().into();
    ServerMessage::GameState {
        game_id,
        board: db.board,
        current_side: db.current_side,
        moves: db.moves,
        result: db.result,
    }
}

#[cfg(feature = "server")]
#[tracing::instrument(skip_all, fields(game_id))]
#[get("/api/game/{game_id}")]
pub async fn get_game(game_id: u64) -> Result<taste_db::Game, ServerFnError> {
    let active = get_active_games().await;
    if let Some(entry) = active.get(&game_id) {
        return Ok(entry.clone().into());
    }

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
) -> Result<GameInfo, ServerFnError> {
    #[cfg(feature = "server")]
    {
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
        Ok(GameInfo {
            id: game.id,
            board: game.board,
            current_side: game.current_side,
            moves: game.moves,
            result: game.result,
        })
    }
    #[cfg(not(feature = "server"))]
    {
        unreachable!()
    }
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
    let active = get_active_games().await;
    if let Some(entry) = active.get(&game_id) {
        return Ok(Some(game_to_state_msg(game_id, &entry)));
    }

    let cache = get_cache().await;
    if let Some(game) = cache.get(&game_id).await {
        let g: tafl_game::Game = game.into();
        active.insert(game_id, g.clone());
        return Ok(Some(game_to_state_msg(game_id, &g)));
    }

    let db = get_db().await;
    let mut db = db.lock().await;
    let db_game = taste_db::Game::get_by_id(&mut *db, &game_id).await.map_err(|e| ServerFnError::ServerError {
            message: e.to_string(), code: 0, details: None,
        })?;
    let g: tafl_game::Game = db_game.into();
    active.insert(game_id, g.clone());
    Ok(Some(game_to_state_msg(game_id, &g)))
}

#[cfg(feature = "server")]
#[tracing::instrument(skip_all, fields(game_id, from_sq, to_sq))]
async fn apply_move(game_id: u64, from_sq: u8, to_sq: u8) -> Result<ServerMessage, ServerFnError> {
    let active = get_active_games().await;
    let cache = get_cache().await;

    // Fast path: load from in-memory state (zero-DB hot path)
    let game = if let Some(entry) = active.get(&game_id) {
        entry.clone()
    } else {
        // Cold start: load from cache, then DB
        let db_game = if let Some(g) = cache.get(&game_id).await {
            g
        } else {
            let db = get_db().await;
            let mut db = db.lock().await;
            taste_db::Game::get_by_id(&mut *db, &game_id)
                .await
                .map_err(|e| ServerFnError::ServerError {
                    message: e.to_string(), code: 0, details: None,
                })?
        };
        let g: tafl_game::Game = db_game.into();
        active.insert(game_id, g.clone());
        g
    };

    let from = tafl_game::Pos::from_index(from_sq).ok_or_else(|| ServerFnError::ServerError {
            message: "invalid from".into(), code: 0, details: None,
        })?;
    let to = tafl_game::Pos::from_index(to_sq).ok_or_else(|| ServerFnError::ServerError {
            message: "invalid to".into(), code: 0, details: None,
        })?;

    let new_game = game.make_move(from, to).ok_or_else(|| ServerFnError::ServerError {
            message: "move rejected".into(), code: 0, details: None,
        })?;

    // Update in-memory state immediately
    active.insert(game_id, new_game.clone());

    let db_new: taste_db::Game = new_game.into();
    cache.insert(game_id, db_new.clone()).await;

    // Async DB write — not on the hot path
    use std::sync::atomic::Ordering;
    PENDING_DB_WRITES.fetch_add(1, Ordering::SeqCst);
    let db_for_write = db_new.clone();
    tokio::spawn(async move {
        let db = get_db().await;
        let mut db = db.lock().await;
        if let Ok(mut db_game) = taste_db::Game::get_by_id(&mut *db, &game_id).await {
            let _ = toasty::update!(db_game {
                board: db_for_write.board.as_str(),
                moves: db_for_write.moves.as_str(),
                current_side: db_for_write.current_side.as_str(),
                result: db_for_write.result.as_str(),
            })
            .exec(&mut *db)
            .await;
        }
        PENDING_DB_WRITES.fetch_sub(1, Ordering::SeqCst);
    });

    // println!("Move {from_sq}->{to_sq} applied to game {game_id}");

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

#[get("/api/ws")]
pub async fn ws(options: WebSocketOptions) -> Result<Websocket<ClientMessage, ServerMessage>> {
    Ok(options.on_upgrade(move |socket| async move {
        use futures::{SinkExt, StreamExt};
        let (mut sender, mut receiver) = socket.split();
        loop {
            match receiver.next().await {
                Some(Ok(ClientMessage::Ping { client_time_ms })) => {
                    let server_time_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    sender
                        .send(ServerMessage::Pong {
                            client_time_ms,
                            server_time_ms,
                        })
                        .await
                        .ok();
                }
                Some(Ok(_)) => {}
                Some(Err(_)) | None => break,
            }
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

#[cfg(all(test, feature = "server"))]
mod bench {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::time::Instant;

    static BENCH_INITED: AtomicBool = AtomicBool::new(false);

    fn generate_move_sequence(count: usize) -> Vec<(u8, u8)> {
        let mut game = tafl_game::Game::new();
        let mut seq = Vec::with_capacity(count);
        for _ in 0..count {
            let moves = game.all_legal_moves();
            if moves.is_empty() {
                break;
            }
            let current_side = game.current_side;
            let found = moves.iter().find(|(pos, _)| {
                let idx = pos.index() as usize;
                match game.board.squares[idx] {
                    tafl_game::P::King | tafl_game::P::Defender => {
                        current_side == tafl_game::Side::Defender
                    }
                    tafl_game::P::Attacker => current_side == tafl_game::Side::Attacker,
                    _ => false,
                }
            });
            let Some((&from_pos, targets)) = found else {
                break;
            };
            let &to_pos = targets.first().unwrap();
            seq.push((from_pos.index(), to_pos.index()));
            game = game.make_move(from_pos, to_pos).unwrap();
        }
        seq
    }

    async fn init_bench_db(n: usize) -> Vec<u64> {
        if !BENCH_INITED.swap(true, Ordering::SeqCst) {
            std::env::set_var("DATABASE_URL", "sqlite::memory:");
            let _ = get_cache().await;
            let _ = get_db().await;
        }
        let db = get_db().await;
        let mut db = db.lock().await;
        let mut ids = Vec::with_capacity(n);
        for _ in 0..n {
            let board = tafl_game::Game::new()
                .ten()
                .split(' ')
                .next()
                .unwrap()
                .to_string();
            let game = toasty::create!(taste_db::Game {
                board: board.as_str(),
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
            .exec(&mut *db)
            .await
            .unwrap();
            ids.push(game.id);
        }
        ids
    }

    /// Measures pure hot-path (no DB write wait) — this is the user-perceived latency.
    async fn bench_hot_path_light(label: &str, game_id: u64, seq: &[(u8, u8)]) -> f64 {
        let n = seq.len();
        if n < 2 {
            return 0.0;
        }
        // Cold-start move (loads from DB/cache, also triggers 1 async DB write)
        let _ = apply_move(game_id, seq[0].0, seq[0].1).await.unwrap();
        // Drain the cold-start background write so the hot path starts clean
        wait_for_pending_writes().await;

        let start = Instant::now();
        for &(from, to) in &seq[1..] {
            apply_move(game_id, from, to).await.unwrap();
        }
        // Do NOT wait for background writes here — measure the hot path only
        let elapsed = start.elapsed();
        let n_hot = n - 1;
        let tput = n_hot as f64 / elapsed.as_secs_f64();
        eprintln!(
            "  {label}: {n_hot} moves in {elapsed:?}  |  {tput:.0} moves/s  |  {:.1} µs/move  (hot path only)",
            1_000_000.0 / tput
        );
        tput
    }

    #[tokio::test]
    #[ignore]
    async fn run_all_benches() {
        eprintln!("\n═══ BENCH 1: Engine only (pure game logic) ═══");
        {
            let mut game = tafl_game::Game::new();
            let mut total = 0;
            let start = Instant::now();
            loop {
                let moves = game.all_legal_moves();
                if moves.is_empty() {
                    break;
                }
                let side = game.current_side;
                let found = moves.iter().find(|(pos, _)| {
                    let idx = pos.index() as usize;
                    match game.board.squares[idx] {
                        tafl_game::P::King | tafl_game::P::Defender => {
                            side == tafl_game::Side::Defender
                        }
                        tafl_game::P::Attacker => side == tafl_game::Side::Attacker,
                        _ => false,
                    }
                });
                let Some((&from, targets)) = found else {
                    break;
                };
                let &to = targets.first().unwrap();
                game = game.make_move(from, to).unwrap();
                total += 1;
            }
            let elapsed = start.elapsed();
            let tput = total as f64 / elapsed.as_secs_f64();
            eprintln!(
                "  {total} moves in {elapsed:?}  |  {tput:.0} moves/s  |  {:.1} ns/move",
                1_000_000_000.0 / tput
            );
        }

        eprintln!("\n═══ BENCH 2: Single game — hot path (user-perceived latency) ═══");
        let ids = init_bench_db(1).await;
        let seq = generate_move_sequence(30);
        let hot_tput = bench_hot_path_light("Single game", ids[0], &seq).await;

        // Now drain to measure total system throughput (hot path + background DB writes)
        wait_for_pending_writes().await;
        // Re-measure same game with same sequence (now ALL moves are hot-path, no cold start)
        let ids2 = init_bench_db(1).await;
        let seq2 = generate_move_sequence(20);
        let _cold = apply_move(ids2[0], seq2[0].0, seq2[0].1).await.unwrap();
        wait_for_pending_writes().await;
        let sstart = Instant::now();
        for &(from, to) in &seq2[1..] {
            apply_move(ids2[0], from, to).await.unwrap();
        }
        wait_for_pending_writes().await; // include DB write
        let selapsed = sstart.elapsed();
        let sn = seq2.len() - 1;
        let stput = sn as f64 / selapsed.as_secs_f64();
        eprintln!(
            "  Single game end-to-end: {sn} moves in {selapsed:?}  |  {stput:.0} moves/s (includes DB write)"
        );
        eprintln!(
            "  Hot path / end-to-end ratio: {:.1}x (DB write overhead)",
            hot_tput / stput
        );

        let n_games = 4;
        eprintln!("\n═══ BENCH 3: {n_games} concurrent games — hot path ═══");
        let cids = init_bench_db(n_games).await;

        // Pre-warm all games
        let mut warmed = Vec::with_capacity(n_games);
        for &gid in &cids {
            let seq = generate_move_sequence(10);
            apply_move(gid, seq[0].0, seq[0].1).await.unwrap();
            warmed.push((gid, seq));
        }
        wait_for_pending_writes().await;

        // Measure concurrent hot-path throughput
        let start = Instant::now();
        let mut handles = Vec::with_capacity(n_games);
        for (gid, seq) in &warmed {
            let gid = *gid;
            let moves: Vec<(u8, u8)> = seq[1..].to_vec();
            handles.push(tokio::spawn(async move {
                for &(from, to) in &moves {
                    apply_move(gid, from, to).await.unwrap();
                }
                moves.len()
            }));
        }
        let mut total_hot = 0;
        for h in handles {
            total_hot += h.await.unwrap();
        }
        let elapsed_concurrent_hot = start.elapsed();
        let tput_concurrent = total_hot as f64 / elapsed_concurrent_hot.as_secs_f64();
        eprintln!(
            "  {total_hot} moves in {elapsed_concurrent_hot:?}  |  {tput_concurrent:.0} moves/s (hot path only)"
        );

        wait_for_pending_writes().await;
        let elapsed_concurrent_total = start.elapsed();
        let tput_concurrent_total = total_hot as f64 / elapsed_concurrent_total.as_secs_f64();
        eprintln!(
            "  {total_hot} moves in {elapsed_concurrent_total:?}  |  {tput_concurrent_total:.0} moves/s (+ DB write)"
        );

        let speedup_hot = tput_concurrent / hot_tput;
        let speedup_total = tput_concurrent_total / stput;
        eprintln!(
            "  Speedup (hot): {speedup_hot:.2}x  |  Speedup (end-to-end): {speedup_total:.2}x  (ideal={n_games}x)"
        );
    }
}
