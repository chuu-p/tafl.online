use crate::views::icons::{Crown, Stone, X};
use dioxus::prelude::*;
use tafl_game::Game;

#[component]
pub fn Board(
    game: Option<Game>,
    scale: f64,
    selected_moves: Vec<(usize, usize)>,
    captured_positions: Vec<(usize, usize)>,
    last_move: Option<((u8, u8), (u8, u8))>,
    on_square_click: EventHandler<(usize, usize)>,
    on_drag_drop: EventHandler<((usize, usize), (usize, usize))>,
) -> Element {
    let sz = 11.0 * scale;
    let icon_sz = (9.0 * scale).ceil();
    let marker_size = (15.0 * scale).ceil();
    let dot_r = 4.0 * scale / 2.0;
    let mut drag_src = use_signal(|| None::<(usize, usize)>);
    let _draggable_positions: std::collections::HashSet<(usize, usize)> = game
        .as_ref()
        .map(|g| {
            g.all_legal_moves()
                .keys()
                .map(|pos| {
                    let i = pos.index() as usize;
                    (i % 9, i / 9)
                })
                .collect()
        })
        .unwrap_or_default();

    rsx! {
        div {
            style: format!("width: {}px; height: {}px; position: relative; display: block;", sz * 9.0, sz * 9.0),
            onmouseup: move |_| { drag_src.set(None); },
            onmousemove: move |_ev| {},
            for file in 0..9 {
                for rank in 0..9 {
                    div {
                        style: format!("position: absolute; width: {}px; height: {}px; background-color: {}; border: 1px solid #f0f6f0; transform: translate({}px, {}px);",
                            sz, sz,
                            if captured_positions.contains(&(file, rank)) { "#f0f6f0" } else {
                                match last_move {
                                    Some(((fr, fc), (tr, tc))) if (fc as usize == file && fr as usize == rank)
                                        || (tc as usize == file && tr as usize == rank) => "#3a3b3b",
                                    _ => "transparent",
                                }
                            },
                            file as f64 * sz, rank as f64 * sz),
                        onmouseup: move |_| {
                            let src = *drag_src.peek();
                            if let Some((f, r)) = src {
                                on_drag_drop.call(((f, r), (file, rank)));
                                drag_src.set(None);
                            } else {
                                on_square_click.call((file, rank));
                            }
                        },
                    }
                }
            }
            for file in 0..9 {
                for rank in 0..9 {
                    { if (file == 0 || file == 8) && (rank == 0 || rank == 8) {
                        rsx! {
                            X {
                                style: format!("position: absolute; pointer-events: none; transform: translate({}px, {}px);",
                                    file as f64 * sz + (sz - marker_size) / 2.0,
                                    rank as f64 * sz + (sz - marker_size) / 2.0),
                                size: marker_size as u32,
                                stroke: "#f0f6f0",
                                stroke_width: 1,
                            }
                        }
                    } else if file == 4 && rank == 4 {
                        rsx! {
                            X {
                                style: format!("position: absolute; pointer-events: none; transform: translate({}px, {}px);",
                                    file as f64 * sz + (sz - marker_size) / 2.0,
                                    rank as f64 * sz + (sz - marker_size) / 2.0),
                                size: marker_size as u32,
                                stroke: "#f0f6f0",
                                stroke_width: 1,
                            }
                        }
                    } else { rsx! {} }}
                }
            }
            { if let Some(ref g) = game {
                rsx! {
                    for file in 0..9 {
                        for rank in 0..9 {
                            { match g.board.squares[rank * 9 + file] {
                                tafl_game::P::King => rsx! {
                                    div {
                                        style: format!("position: absolute; width: {}px; height: {}px; cursor: grab; user-select: none; -webkit-user-select: none; transform: translate({}px, {}px);",
                                            icon_sz, icon_sz,
                                            file as f64 * sz + (sz - icon_sz) / 2.0,
                                            rank as f64 * sz + (sz - icon_sz) / 2.0),
                                        onmousedown: move |_| { drag_src.set(Some((file, rank))); on_square_click.call((file, rank)); },
                                        onclick: move |_| on_square_click.call((file, rank)),
                                        Crown { size: icon_sz as u32, fill: "#f0f6f0", stroke: "#222323", stroke_width: 1.5 }
                                    }
                                },
                                tafl_game::P::Defender => rsx! {
                                    div {
                                        style: format!("position: absolute; width: {}px; height: {}px; cursor: grab; user-select: none; -webkit-user-select: none; transform: translate({}px, {}px);",
                                            icon_sz, icon_sz,
                                            file as f64 * sz + (sz - icon_sz) / 2.0,
                                            rank as f64 * sz + (sz - icon_sz) / 2.0),
                                        onmousedown: move |_| { drag_src.set(Some((file, rank))); on_square_click.call((file, rank)); },
                                        onclick: move |_| on_square_click.call((file, rank)),
                                        Stone { size: icon_sz as u32, fill: "#f0f6f0", stroke: "#222323", stroke_width: 1.5 }
                                    }
                                },
                                tafl_game::P::Attacker => rsx! {
                                    div {
                                        style: format!("position: absolute; width: {}px; height: {}px; cursor: grab; user-select: none; -webkit-user-select: none; transform: translate({}px, {}px);",
                                            icon_sz, icon_sz,
                                            file as f64 * sz + (sz - icon_sz) / 2.0,
                                            rank as f64 * sz + (sz - icon_sz) / 2.0),
                                        onmousedown: move |_| { drag_src.set(Some((file, rank))); on_square_click.call((file, rank)); },
                                        onclick: move |_| on_square_click.call((file, rank)),
                                        Stone { size: icon_sz as u32, fill: "#222323", stroke: "#f0f6f0", stroke_width: 1.5 }
                                    }
                                },
                                tafl_game::P::Empty => rsx! {},
                            }}
                        }
                    }
                }
            } else { rsx! {} }}
            for &(mf, mr) in &selected_moves {
                div {
                    style: format!("position: absolute; width: {}px; height: {}px; border-radius: 50%; background-color: #f0f6f0; opacity: 0.5; pointer-events: none; transform: translate({}px, {}px);",
                        dot_r * 2.0, dot_r * 2.0,
                        mf as f64 * sz + (sz - dot_r * 2.0) / 2.0,
                        mr as f64 * sz + (sz - dot_r * 2.0) / 2.0),
                }
            }
        }
    }
}
