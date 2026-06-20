use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tafl_domain::board::{Board, Piece, Side, SIZE};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub board: [[Piece; SIZE]; SIZE],
    pub turn: Side,
    pub players: HashMap<Side, String>,
    pub messages: Vec<GameMessage>,
    pub winner: Option<Side>,
    pub white_time: u32,
    pub black_time: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMessage {
    pub kind: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRequest {
    pub side: Side,
    pub from: (usize, usize),
    pub to: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRequest {
    pub variant: String,
    pub minutes: u32,
    pub increment: u32,
}

// ---------------------------------------------------------------------------
// Room
// ---------------------------------------------------------------------------

pub struct Room {
    pub board: Board,
    pub turn: Side,
    pub players: HashMap<Side, String>,
    pub messages: Vec<GameMessage>,
    pub winner: Option<Side>,
    pub white_time: u32,
    pub black_time: u32,
    pub increment: u32,
}

impl Room {
    fn new(minutes: u32, increment: u32) -> Self {
        Room {
            board: Board::new(),
            turn: Side::Swedes,
            players: HashMap::new(),
            messages: vec![],
            winner: None,
            white_time: minutes * 60,
            black_time: minutes * 60,
            increment,
        }
    }

    fn board_array(&self) -> [[Piece; SIZE]; SIZE] {
        let mut arr = [[Piece::Empty; SIZE]; SIZE];
        for r in 0..SIZE {
            for c in 0..SIZE {
                arr[r][c] = self.board.get(r, c);
            }
        }
        arr
    }

    fn state(&self) -> GameState {
        GameState {
            board: self.board_array(),
            turn: self.turn,
            players: self.players.clone(),
            messages: self.messages.clone(),
            winner: self.winner,
            white_time: self.white_time,
            black_time: self.black_time,
        }
    }

    fn push_msg(&mut self, kind: &str, data: serde_json::Value) {
        self.messages.push(GameMessage {
            kind: kind.to_string(),
            data,
        });
    }
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

pub type SharedState = Arc<RwLock<HashMap<String, Room>>>;

pub fn new_state() -> SharedState {
    Arc::new(RwLock::new(HashMap::new()))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn create_handler(
    Extension(state): Extension<SharedState>,
    Json(req): Json<CreateRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = format!("{:08x}", rand::random::<u32>());
    let room = Room::new(req.minutes, req.increment);
    state.write().await.insert(id.clone(), room);
    Ok(Json(serde_json::json!({ "room_id": id })))
}

pub async fn join_handler(
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<JoinRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut state = state.write().await;
    let room = state.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;

    if room.winner.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let side = if !room.players.contains_key(&Side::Swedes) {
        Side::Swedes
    } else if !room.players.contains_key(&Side::Muscovites) {
        Side::Muscovites
    } else {
        return Err(StatusCode::BAD_REQUEST); // full
    };

    room.players.insert(side, req.name.clone());
    room.push_msg("join", serde_json::json!({ "side": side, "name": req.name }));

    if room.players.len() == 2 {
        room.push_msg("start", serde_json::json!({ "turn": room.turn }));
    }

    Ok(Json(serde_json::json!({ "side": side })))
}

pub async fn move_handler(
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<MoveRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut state = state.write().await;
    let room = state.get_mut(&id).ok_or((StatusCode::NOT_FOUND, "Room not found".into()))?;

    if room.winner.is_some() {
        return Err((StatusCode::BAD_REQUEST, "Game is over".into()));
    }

    if room.turn != req.side {
        return Err((StatusCode::BAD_REQUEST, format!("Not your turn. Turn: {:?}", room.turn)));
    }

    if !room.board.is_valid_move(req.from.0, req.from.1, req.to.0, req.to.1) {
        return Err((StatusCode::BAD_REQUEST, "Invalid move".into()));
    }

    room.board.make_move(req.from.0, req.from.1, req.to.0, req.to.1);
    let captures = room.board.find_captures_with_castle(req.to.0, req.to.1, req.side);
    room.board.remove_captures(&captures);

    // Apply increment
    match req.side {
        Side::Swedes => room.white_time += room.increment,
        Side::Muscovites => room.black_time += room.increment,
    }

    // Check win
    let win = room.board.check_win();
    if let Some(winner) = win {
        room.winner = Some(winner);
        room.push_msg("gameover", serde_json::json!({ "winner": winner }));
    } else {
        room.turn = req.side.opposite();
    }

    // Check warning
    if let Some(warning) = room.board.detect_warning() {
        room.push_msg("warning", serde_json::json!({ "kind": warning }));
    }

    room.push_msg(
        "move",
        serde_json::json!({
            "from": req.from,
            "to": req.to,
            "side": req.side,
            "captures": captures,
        }),
    );

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn state_handler(
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<GameState>, StatusCode> {
    let state = state.read().await;
    let room = state.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(room.state()))
}

pub async fn messages_handler(
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, usize>>,
) -> Result<Json<Vec<GameMessage>>, StatusCode> {
    let since = params.get("since").copied().unwrap_or(0);
    let state = state.read().await;
    let room = state.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let msgs = room.messages[since..].to_vec();
    Ok(Json(msgs))
}

use axum::extract::Query;

pub async fn resign_handler(
    Extension(state): Extension<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let side: Side = serde_json::from_value(
        req.get("side")
            .cloned()
            .ok_or(StatusCode::BAD_REQUEST)?,
    )
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut state = state.write().await;
    let room = state.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;

    if room.winner.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    room.winner = Some(side.opposite());
    room.push_msg("gameover", serde_json::json!({ "winner": side.opposite() }));

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub fn router() -> Router {
    Router::new()
        .route("/api/game/create", axum::routing::post(create_handler))
        .route("/api/game/{id}/join", axum::routing::post(join_handler))
        .route("/api/game/{id}/move", axum::routing::post(move_handler))
        .route("/api/game/{id}/state", axum::routing::get(state_handler))
        .route("/api/game/{id}/messages", axum::routing::get(messages_handler))
        .route("/api/game/{id}/resign", axum::routing::post(resign_handler))
        .route("/api/game/list", axum::routing::get(list_handler))
}

pub async fn list_handler(
    Extension(state): Extension<SharedState>,
) -> Json<Vec<serde_json::Value>> {
    let state = state.read().await;
    let games = state
        .iter()
        .filter(|(_, room)| room.players.len() < 2 && room.winner.is_none())
        .map(|(id, room)| {
            let white = room.players.get(&Side::Swedes).cloned().unwrap_or_default();
            serde_json::json!({
                "room_id": id,
                "variant": "tablut",
                "white": white,
                "minutes": room.white_time / 60,
                "increment": room.increment,
            })
        })
        .collect();
    Json(games)
}
