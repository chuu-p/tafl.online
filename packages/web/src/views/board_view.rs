use api::{ClientMessage, ServerMessage};
use dioxus::fullstack::{use_websocket, WebSocketOptions};
use dioxus::prelude::*;
use tafl_game::Game;

use crate::views::Board;

#[component]
pub fn GameView(id: String) -> Element {
    rsx! { GameViewInner { gid: id.clone(), player_side: None } }
}

#[component]
pub fn GameVs(id: String, side: String) -> Element {
    rsx! { GameViewInner { gid: id, player_side: Some(side) } }
}

#[component]
fn GameViewInner(gid: String, player_side: Option<String>) -> Element {
    let mut game = use_signal(|| None::<Game>);
    let selected_from = use_signal(|| None::<(usize, usize)>);
    let selected_moves = use_signal(|| vec![]);
    let captured_positions = use_signal(|| vec![]);
    let mut qr_svg = use_signal(|| None::<String>);
    let mut dark = use_signal(|| false);

    let ws_id: u64 = gid.parse().unwrap_or(0);
    #[cfg(feature = "web")]
    web_sys::console::log_1(&format!("[gamews] connecting to game {ws_id}").into());
    let socket = use_websocket(move || api::get_game_ws(ws_id, WebSocketOptions::new()));

    let mut ws_recv = socket;
    use_future(move || async move {
        while let Ok(msg) = ws_recv.recv().await {
            match msg {
                ServerMessage::GameState {
                    board,
                    current_side,
                    moves,
                    result,
                    game_id: _,
                } => {
                    #[cfg(feature = "web")]
                    web_sys::console::log_1(&format!("[gamews] game_state board={board:0.30}.. side={current_side} result={result} moves={moves:0.30}..").into());
                    let full_ten = format!("{} {} - 0 1", board, current_side);
                    let mut g = Game::from_ten(&full_ten);
                    g.moves = serde_json::from_str(&moves).unwrap_or_default();
                    g.result = match result.as_str() {
                        "a" => tafl_game::Result::AttackerWin,
                        "d" => tafl_game::Result::DefenderWin,
                        "=" => tafl_game::Result::Draw,
                        _ => tafl_game::Result::Pending,
                    };
                    game.set(Some(g));
                }
                ServerMessage::Pong { .. } => {
                    #[cfg(feature = "web")]
                    web_sys::console::log_1(&"[gamews] pong".into());
                }
                ServerMessage::MoveResult { accepted, from_sq, to_sq } => {
                    #[cfg(feature = "web")]
                    web_sys::console::log_1(&format!("[gamews] move_result accepted={accepted} from={from_sq} to={to_sq}").into());
                }
                ServerMessage::Error { message } => {
                    #[cfg(feature = "web")]
                    web_sys::console::error_1(&format!("[gamews] error: {message}").into());
                }
            }
        }
    });

    let side_for_click = player_side.clone();
    let on_square_click = {
        let mut sf = selected_from;
        let mut sm = selected_moves;
        let mut sc = captured_positions;
        let g = game;
        let socket = socket;
        move |(file, rank): (usize, usize)| {
            let Some(ref current_g) = g() else { return };
            let click_idx = rank * 9 + file;
            let clicked_piece = current_g.board.squares[click_idx];
            let current_moves = sm();
            let current_from = sf();

            if current_moves.contains(&(file, rank)) {
                if let Some((from_f, from_r)) = current_from {
                    let from_idx = from_r * 9 + from_f;
                    sf.set(None);
                    sm.set(vec![]);
                    sc.set(vec![]);
                    #[cfg(feature = "web")]
                    web_sys::console::log_1(&format!("[gamews] sending move from={from_idx} to={click_idx}").into());
                    spawn(async move {
                        socket
                            .send(ClientMessage::MakeMove {
                                from_sq: from_idx as u8,
                                to_sq: click_idx as u8,
                            })
                            .await
                            .ok();
                    });
                }
                return;
            }

            let is_current_turn_piece = match (
                clicked_piece,
                current_g.current_side,
                side_for_click.as_deref(),
            ) {
                (tafl_game::P::King | tafl_game::P::Defender, tafl_game::Side::Defender, None) => {
                    true
                }
                (tafl_game::P::Attacker, tafl_game::Side::Attacker, None) => true,
                (
                    tafl_game::P::King | tafl_game::P::Defender,
                    tafl_game::Side::Defender,
                    Some("defender"),
                ) => true,
                (tafl_game::P::Attacker, tafl_game::Side::Attacker, Some("attacker")) => true,
                _ => false,
            };
            if is_current_turn_piece {
                let Some(pos) = tafl_game::Pos::from_index(click_idx as u8) else {
                    return;
                };
                let all_moves = current_g.all_legal_moves();
                let targets = all_moves
                    .get(&pos)
                    .map(|v| {
                        v.iter()
                            .map(|p| {
                                let i = p.index() as usize;
                                (i % 9, i / 9)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let captures: Vec<(usize, usize)> = all_moves
                    .get(&pos)
                    .into_iter()
                    .flat_map(|moves| {
                        moves.iter().flat_map(|target| {
                            current_g
                                .board
                                .get_captured_positions(pos, *target)
                                .into_iter()
                                .map(|(row, col)| (col, row))
                        }).collect::<Vec<_>>()
                    })
                    .collect();
                sf.set(Some((file, rank)));
                sm.set(targets);
                sc.set(captures);
            } else {
                sf.set(None);
                sm.set(vec![]);
                sc.set(vec![]);
            }
        }
    };

    let drag_socket = socket;
    let on_drag_drop = {
        let mut sf = selected_from;
        let mut sm = selected_moves;
        let mut sc = captured_positions;
        move |(from, to): ((usize, usize), (usize, usize))| {
            sf.set(None);
            sm.set(vec![]);
            sc.set(vec![]);
            let from_idx = from.1 * 9 + from.0;
            let to_idx = to.1 * 9 + to.0;
            spawn(async move {
                drag_socket
                    .send(ClientMessage::MakeMove {
                        from_sq: from_idx as u8,
                        to_sq: to_idx as u8,
                    })
                    .await
                    .ok();
            });
        }
    };

    let last_move = game()
        .and_then(|g| g.moves.last().cloned())
        .map(|m| (m.from, m.to));

    let turn_text = game()
        .map(|g| match g.current_side {
            tafl_game::Side::Attacker => "Attackers".to_string(),
            tafl_game::Side::Defender => "Defenders".to_string(),
        })
        .unwrap_or_default();
    let result_text = game()
        .map(|g| match g.result {
            tafl_game::Result::Pending => String::new(),
            tafl_game::Result::AttackerWin => " — Attackers Win".to_string(),
            tafl_game::Result::DefenderWin => " — Defenders Win".to_string(),
            tafl_game::Result::Draw => " — Draw".to_string(),
        })
        .unwrap_or_default();

    let fen = game().map(|g| g.ten()).unwrap_or_default();
    let tpgn = game()
        .as_ref()
        .map(|g| {
            g.moves
                .chunks(2)
                .enumerate()
                .map(|(i, pair)| {
                    let a1 = |&(r, c): &(u8, u8)| format!("{}{}", (b'a' + r) as char, c + 1);
                    let m1 = format!("{}. {} {}", i + 1, a1(&pair[0].from), a1(&pair[0].to));
                    if pair.len() > 1 {
                        format!("{} {} {}", m1, a1(&pair[1].from), a1(&pair[1].to))
                    } else {
                        m1
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();

    rsx! {
        div {
            id: "game",
            p { "Game #{gid}" }
            div { style: "display: flex; align-items: center; gap: 10px;",
                span { "To move: {turn_text}{result_text}" }
                { if let Some(ref s) = player_side {
                    let opponent_side = if s == "attacker" { "defender" } else { "attacker" };
                    let origin = {
                        #[cfg(feature = "web")]
                        { js_sys::eval("window.location.origin").ok().and_then(|v| v.as_string()).unwrap_or_default() }
                        #[cfg(not(feature = "web"))]
                        { String::new() }
                    };
                    let link = format!("{}/game/{}/{}", origin, gid, opponent_side);
                    let copy_link = link.clone();
                    let qr_link = link.clone();
                    rsx! {
                        button {
                            onclick: move |_| {
                                #[cfg(feature = "web")]
                                let _ = js_sys::eval(&format!("navigator.clipboard.writeText('{}')", copy_link.replace('\'', "\\'")));
                            },
                            "Copy opponent link"
                        }
                        button {
                            onclick: move |_| {
                                let svg = fast_qr::QRBuilder::new(qr_link.as_str())
                                    .build()
                                    .ok()
                                    .map(|qr| fast_qr::convert::svg::SvgBuilder::default().to_str(&qr).replacen("<svg", "<svg width=\"100%\" height=\"100%\"", 1));
                                qr_svg.set(svg);
                            },
                            "QR opponent link"
                        }
                    }
                } else { rsx! {} }}
                button {
                    onclick: move |_| dark.set(!dark()),
                    if dark() { "Light" } else { "Dark" }
                }
            }
            div {
                style: format!("filter:invert({})", if dark() { 1 } else { 0 }),
                Board { game: game(), scale: 4.0, selected_moves: selected_moves(), captured_positions: captured_positions(), last_move, on_square_click, on_drag_drop }
            }
            div { style: "margin-top: 10px; font-family: monospace; font-size: 0.8rem;",
                div { "FEN: {fen}" }
                div { "TPGN: {tpgn}" }
            }
            { if let Some(ref svg) = *qr_svg.read() {
                rsx! {
                    div {
                        style: "position:fixed;inset:0;background:rgba(0,0,0,0.5);display:flex;align-items:center;justify-content:center;z-index:1000",
                        onclick: move |_| qr_svg.set(None),
                        div {
                            style: "background:#222323;padding:20px;border-radius:0;max-width:300px;border:1px solid #f0f6f0",
                            onclick: move |e| e.stop_propagation(),
                            div { dangerous_inner_html: svg.clone() }
                        }
                    }
                }
            } else { rsx! {} }}
        }
    }
}
