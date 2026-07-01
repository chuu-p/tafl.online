use api::ping_ws;
use dioxus::fullstack::{use_websocket, WebSocketOptions};
use dioxus::prelude::*;

#[component]
pub fn WsPing() -> Element {
    let mut socket = use_websocket(|| ping_ws(WebSocketOptions::new()));
    let mut response = use_signal(|| String::new());

    use_future(move || async move {
        while let Ok(msg) = socket.recv().await {
            response.set(msg);
        }
    });

    rsx! {
        div {
            h4 { "WebSocket Ping" }
            input {
                placeholder: "Type to ping...",
                oninput: move |_| async move {
                    #[cfg(feature = "web")]
                    let now = js_sys::Date::now() as u64;
                    #[cfg(not(feature = "web"))]
                    let now = 0;
                    _ = socket.send(now).await;
                },
            }
            if !response().is_empty() {
                p { i { "{response}" } }
            }
        }
    }
}
