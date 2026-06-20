#[cfg(feature = "server")]
mod auth;
#[cfg(feature = "server")]
mod game;

use dioxus::prelude::*;
use serde_json::json;

#[derive(Debug, Clone, Routable, PartialEq)]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/game/:room_id")]
    Game { room_id: String },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    #[cfg(feature = "server")]
    {
        let _ = dotenvy::dotenv();

        dioxus_server::serve(|| async move {
            use dioxus_server::{DioxusRouterExt, ServeConfig};

            let redirect_uri = std::env::var("OAUTH_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:8080/api/auth/callback".to_string());

            let auth_state = auth::AuthState::new(redirect_uri);
            let game_state = game::new_state();
            let config = ServeConfig::default();

            let app = axum::Router::new()
                .route("/api/auth/login", axum::routing::get(auth::login_handler))
                .route("/api/auth/callback", axum::routing::get(auth::callback_handler))
                .route("/api/auth/me", axum::routing::get(auth::me_handler))
                .route("/api/auth/logout", axum::routing::get(auth::logout_handler))
                .route("/api/game/create", axum::routing::post(game::create_handler))
                .route("/api/game/list", axum::routing::get(game::list_handler))
                .route("/api/game/{id}/join", axum::routing::post(game::join_handler))
                .route("/api/game/{id}/move", axum::routing::post(game::move_handler))
                .route("/api/game/{id}/state", axum::routing::get(game::state_handler))
                .route("/api/game/{id}/messages", axum::routing::get(game::messages_handler))
                .route("/api/game/{id}/resign", axum::routing::post(game::resign_handler))
                .layer(tower_cookies::CookieManagerLayer::new())
                .layer(axum::extract::Extension(auth_state))
                .layer(axum::extract::Extension(game_state))
                .serve_dioxus_application(config, App);

            Ok(app)
        });
    }

    #[cfg(not(feature = "server"))]
    {
        dioxus::launch(App);
    }
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
struct MeResponse {
    name: String,
}

#[component]
fn Navbar() -> Element {
    let mut logged_in = use_signal(|| false);
    let mut user_name = use_signal(|| String::new());
    let mut show_play = use_signal(|| false);
    let mut minutes = use_signal(|| 5);
    let mut increment = use_signal(|| 3);

    use_effect(move || {
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                let window = web_sys::window().unwrap();
                let resp_val = wasm_bindgen_futures::JsFuture::from(
                    window.fetch_with_str("/api/auth/me"),
                )
                .await
                .unwrap();
                let resp: web_sys::Response = resp_val.dyn_into().unwrap();
                if resp.status() == 200 {
                    let text_val = wasm_bindgen_futures::JsFuture::from(resp.text().unwrap())
                        .await
                        .unwrap();
                    if let Some(text) = text_val.as_string() {
                        if let Ok(info) = serde_json::from_str::<MeResponse>(&text) {
                            logged_in.set(true);
                            user_name.set(info.name);
                        }
                    }
                }
            }
        });
    });

    let create_game = move |_| {
        let m = *minutes.read();
        let inc = *increment.read();
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                let body = serde_json::json!({
                    "variant": "tablut",
                    "minutes": m,
                    "increment": inc,
                });
                let opts = web_sys::RequestInit::new();
                opts.set_method("POST");
                let body_val = wasm_bindgen::JsValue::from_str(&body.to_string());
                opts.set_body(&body_val);
                let headers = web_sys::Headers::new().unwrap();
                headers.append("Content-Type", "application/json").unwrap();
                opts.set_headers(&headers.into());

                let window = web_sys::window().unwrap();
                let req = web_sys::Request::new_with_str_and_init("/api/game/create", &opts).unwrap();
                let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&req))
                    .await
                    .unwrap();
                let resp: web_sys::Response = resp_val.dyn_into().unwrap();
                if resp.status() == 200 {
                    let text_val = wasm_bindgen_futures::JsFuture::from(resp.text().unwrap())
                        .await
                        .unwrap();
                    if let Some(text) = text_val.as_string() {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(rid) = val.get("room_id").and_then(|v| v.as_str()) {
                                window.location().set_href(&format!("/game/{}", rid)).unwrap();
                            }
                        }
                    }
                }
            }
        });
        show_play.set(false);
    };

    rsx! {
        nav { class: "navbar",
            div { class: "navbar-left",
                Link { to: Route::Home {}, class: "brand", "tafl.online" }
                a {
                    href: "#",
                    class: "nav-link play-link",
                    onclick: move |e| { e.prevent_default(); show_play.set(true); },
                    "Play"
                }
            }
            div { class: "navbar-right",
                if *logged_in.read() {
                    span { class: "nav-user", "{user_name}" }
                    a { href: "/api/auth/logout", class: "nav-link", "Sign out" }
                } else {
                    a { href: "/api/auth/login", class: "nav-link", "Sign in" }
                }
            }
        }
        if *show_play.read() {
            div { class: "modal-overlay",
                onclick: move |_| show_play.set(false),
                div { class: "modal", onclick: move |e| e.stop_propagation(),
                    h3 { "Create Game" }

                    label { class: "field-label", "Variant" }
                    select { class: "field-select", id: "variant",
                        option { value: "tablut", "Tablut" }
                        option { value: "hnefatafl", "Hnefatafl" }
                    }

                    label { class: "field-label", "Minutes per side" }
                    div { class: "slider-row",
                        input {
                            class: "field-slider",
                            r#type: "range",
                            min: "1",
                            max: "120",
                            value: "{minutes}",
                            oninput: move |e| { *minutes.write() = e.value().parse().unwrap_or(5); },
                        }
                        span { class: "val-box", "{minutes}" }
                    }

                    label { class: "field-label", "Increment in seconds" }
                    div { class: "slider-row",
                        input {
                            class: "field-slider",
                            r#type: "range",
                            min: "0",
                            max: "120",
                            value: "{increment}",
                            oninput: move |e| { *increment.write() = e.value().parse().unwrap_or(3); },
                        }
                        span { class: "val-box", "{increment}" }
                    }

                    button { class: "btn-primary", onclick: create_game, "Create lobby game" }
                }
            }
        }
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    let mut games = use_signal(|| Vec::<serde_json::Value>::new());

    use_effect(move || {
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                let window = web_sys::window().unwrap();
                let resp_val = wasm_bindgen_futures::JsFuture::from(
                    window.fetch_with_str("/api/game/list"),
                )
                .await
                .unwrap();
                let resp: web_sys::Response = resp_val.dyn_into().unwrap();
                if resp.status() == 200 {
                    let text_val = wasm_bindgen_futures::JsFuture::from(resp.text().unwrap())
                        .await
                        .unwrap();
                    if let Some(text) = text_val.as_string() {
                        if let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                            games.set(list);
                        }
                    }
                }
            }
        });
    });

    rsx! {
        main { class: "home",
            div { class: "lobby",
                h2 { "Open Games" }
                table { class: "lobby-table",
                    thead {
                        tr {
                            th { "Variant" }
                            th { "Time" }
                            th { "" }
                        }
                    }
                    tbody {
                        if games.read().is_empty() {
                            tr {
                                td { colspan: "3", class: "empty", "No games waiting. Click Play to create one." }
                            }
                        }
                        GameList { games }
                    }
                }
            }
        }
    }
}

#[component]
fn GameList(games: Signal<Vec<serde_json::Value>>) -> Element {
    rsx! {
        for game in games.read().iter() {
            GameRow { game: game.clone() }
        }
    }
}

#[component]
fn GameRow(game: serde_json::Value) -> Element {
    let rid = game.get("room_id").and_then(|v| v.as_str()).unwrap_or("?");
    let variant = game.get("variant").and_then(|v| v.as_str()).unwrap_or("?");
    let minutes = game.get("minutes").and_then(|v| v.as_u64()).unwrap_or(5);
    let increment = game.get("increment").and_then(|v| v.as_u64()).unwrap_or(0);
    let time_str = format!("{}+{}", minutes, increment);
    let href = format!("/game/{}", rid);

    rsx! {
        tr {
            td { "{variant}" }
            td { "{time_str}" }
            td {
                a { href: "{href}", class: "btn-primary", "Join" }
            }
        }
    }
}

#[component]
fn Game(room_id: String) -> Element {
    let mut board = use_signal(|| [[0i32; 9]; 9]);
    let mut turn = use_signal(|| "Swedes".to_string());
    let mut my_side = use_signal(|| String::new());
    let mut selected = use_signal(|| None::<(usize, usize)>);
    let mut status = use_signal(|| "Waiting...".to_string());
    let mut white_time = use_signal(|| 300u32);
    let mut black_time = use_signal(|| 300u32);

    // Auto-join
    let rid = room_id.clone();
    use_effect(move || {
        let rid = rid.clone();
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(v) = fetch_json_post(&format!("/api/game/{}/join", rid), json!({"name":"Player"})).await {
                    if let Some(s) = v.get("side").and_then(|v| v.as_str()) {
                        my_side.set(s.to_string());
                    }
                }
            }
        });
    });

    // Poll state
    let rid = room_id.clone();
    use_effect(move || {
        let rid = rid.clone();
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            loop {
                sleep_ms(500).await;
                if let Some(v) = fetch_json_get(&format!("/api/game/{}/state", rid)).await {
                    if let Some(b) = v.get("board").and_then(|x| x.as_array()) {
                        let mut nb = [[0i32; 9]; 9];
                        for r in 0..9 {
                            if let Some(row) = b[r].as_array() {
                                for c in 0..9 { nb[r][c] = row[c].as_i64().unwrap_or(0) as i32; }
                            }
                        }
                        board.set(nb);
                    }
                    if let Some(t) = v.get("turn") { turn.set(t.as_str().unwrap_or("Swedes").to_string()); }
                    if let Some(w) = v.get("white_time") { white_time.set(w.as_u64().unwrap_or(300) as u32); }
                    if let Some(b) = v.get("black_time") { black_time.set(b.as_u64().unwrap_or(300) as u32); }
                    if let Some(w) = v.get("winner").filter(|v| !v.is_null()) {
                        status.set(format!("{} wins!", w.as_str().unwrap_or("?")));
                    } else if let Some(p) = v.get("players") {
                        if p.as_object().map_or(false, |o| o.len() < 2) {
                            status.set("Waiting...".into());
                        }
                    }
                }
            }
        });
    });

    let rid2 = room_id.clone();
    rsx! {
        main { class: "home",
            div { class: "game-area",
                div { class: "board",
                    for r in 0..9 {
                        BoardRow { row: r, board, selected, my_side: my_side.read().clone(), turn: turn.read().clone(), room_id: rid2.clone() }
                    }
                }
                div { class: "game-info",
                    p { "Room: {room_id}" }
                    p { "{status}" }
                    p { "Turn: {turn}" }
                    p { "W: {white_time}s  B: {black_time}s" }
                    if !my_side.read().is_empty() {
                        p { "You: {my_side}" }
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                let rid = room_id.clone();
                                let side = my_side.read().clone();
                                spawn(async move {
                                    #[cfg(target_arch = "wasm32")]
                                    fetch_json_post(&format!("/api/game/{}/resign", rid), json!({"side": side})).await;
                                });
                            },
                            "Resign"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: i32) {
    use wasm_bindgen::JsCast;
    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(
            wasm_bindgen::closure::Closure::once_into_js(move || {
                resolve.call0(&wasm_bindgen::JsValue::NULL).unwrap();
            }).unchecked_ref(), ms,
        ).unwrap();
    })).await.unwrap();
}

#[cfg(target_arch = "wasm32")]
async fn fetch_json_get(url: &str) -> Option<serde_json::Value> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().unwrap();
    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url)).await.ok()?;
    let resp: web_sys::Response = resp_val.dyn_into().ok()?;
    if resp.status() != 200 { return None; }
    let text_val = wasm_bindgen_futures::JsFuture::from(resp.text().ok()?).await.ok()?;
    let text = text_val.as_string()?;
    serde_json::from_str(&text).ok()
}

#[cfg(target_arch = "wasm32")]
async fn fetch_json_post(url: &str, body: serde_json::Value) -> Option<serde_json::Value> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().unwrap();
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    let body_val = wasm_bindgen::JsValue::from_str(&body.to_string());
    opts.set_body(&body_val);
    let headers = web_sys::Headers::new().unwrap();
    headers.append("Content-Type", "application/json").unwrap();
    opts.set_headers(&headers.into());
    let req = web_sys::Request::new_with_str_and_init(url, &opts).ok()?;
    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&req)).await.ok()?;
    let resp: web_sys::Response = resp_val.dyn_into().ok()?;
    if resp.status() != 200 { return None; }
    let text_val = wasm_bindgen_futures::JsFuture::from(resp.text().ok()?).await.ok()?;
    let text = text_val.as_string()?;
    serde_json::from_str(&text).ok()
}

#[component]
fn BoardRow(
    row: usize,
    board: Signal<[[i32; 9]; 9]>,
    selected: Signal<Option<(usize, usize)>>,
    my_side: String,
    turn: String,
    room_id: String,
) -> Element {
    rsx! {
        div { class: "board-row", key: "{row}",
            for col in 0..9 {
                GameCell {
                    row,
                    col,
                    board,
                    selected,
                    my_side: my_side.clone(),
                    turn: turn.clone(),
                    room_id: room_id.clone(),
                }
            }
        }
    }
}

#[component]
fn GameCell(
    row: usize,
    col: usize,
    mut board: Signal<[[i32; 9]; 9]>,
    mut selected: Signal<Option<(usize, usize)>>,
    my_side: String,
    turn: String,
    room_id: String,
) -> Element {
    let p = board.read()[row][col];
    let is_sel = *selected.read() == Some((row, col));
    let sym = match p { 1 => "●", 2 => "○", 3 => "♚", _ => "" };
    let cls = match (p, is_sel) {
        (1, true) => "cell attacker sel",
        (2, true) => "cell defender sel",
        (3, true) => "cell king sel",
        (1, _) => "cell attacker",
        (2, _) => "cell defender",
        (3, _) => "cell king",
        _ => "cell",
    };

    let on_click = move |_| {
        let cur = board.read()[row][col];
        let sel = *selected.read();
        if let Some((sr, sc)) = sel {
            if (sr, sc) != (row, col) && cur == 0 {
                let rid = room_id.clone();
                let side = my_side.clone();
                spawn(async move {
                    #[cfg(target_arch = "wasm32")]
                    fetch_json_post(&format!("/api/game/{}/move", rid), json!({"side":side,"from":[sr,sc],"to":[row,col]})).await;
                });
                selected.set(None);
            } else if cur != 0 {
                selected.set(Some((row, col)));
            } else {
                selected.set(None);
            }
        } else if cur != 0 {
            let is_own = (my_side == "Swedes" && (cur == 2 || cur == 3))
                      || (my_side == "Muscovites" && cur == 1);
            if is_own && turn == my_side {
                selected.set(Some((row, col)));
            }
        }
    };

    rsx! {
        div { class: "{cls}", key: "{row}-{col}",
            onclick: on_click,
            "{sym}"
        }
    }
}
