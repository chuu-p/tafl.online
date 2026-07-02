use dioxus::prelude::*;

fn log_add(mut logs: Signal<Vec<String>>, msg: &str) {
    #[cfg(feature = "web")]
    web_sys::console::log_1(&msg.into());
    logs.write().push(msg.to_string());
    if logs.read().len() > 50 {
        logs.write().remove(0);
    }
}

#[component]
pub fn Home() -> Element {
    let (_, _, connected, logs) =
        use_context::<(Signal<u64>, Signal<bool>, Signal<bool>, Signal<Vec<String>>)>();
    let mut game_error = use_signal(|| String::new());

    let make_game = move |side: Option<&str>| {
        let side = side.map(|s| s.to_string());
        let logs = logs;
        spawn(async move {
            log_add(logs, "[api] create_game called");
            match api::create_game(None, None, None, None, None, None, None, None).await {
                Ok(game) => {
                    log_add(logs, &format!("[api] create_game success id={}", game.id));
                    game_error.set(String::new());
                    let path = match side {
                        Some(s) => format!("/game/{}/{}", game.id, s),
                        None => format!("/game/{}", game.id),
                    };
                    navigator().push(path);
                }
                Err(e) => {
                    log_add(logs, &format!("[api] create_game failed: {e}"));
                    game_error.set(format!("{e}"));
                }
            }
        });
    };

    rsx! {
        div {
            p {
                style: "font-size: 0.85rem;",
                if connected() { "✓ connected" } else { "✗ disconnected" }
            }
            button {
                onclick: move |_| make_game(None),
                "New Game"
            }
            " "
            button {
                onclick: move |_| make_game(Some("attacker")),
                "New Game as Attacker"
            }
            " "
            button {
                onclick: move |_| make_game(Some("defender")),
                "New Game as Defender"
            }
            if !game_error().is_empty() {
                p { style: "color: #ff6b6b; margin-top: 12px; font-size: 0.85rem;",
                    "{game_error}"
                }
            }
            div {
                style: "margin-top: 20px; padding-top: 12px; border-top: 1px solid #f0f6f0; font-size: 0.75rem; font-family: monospace; white-space: pre-wrap; word-break: break-all; max-height: 400px; overflow-y: auto;",
                for line in logs().iter().rev().take(30) {
                    div { "{line}" }
                }
            }
        }
    }
}
