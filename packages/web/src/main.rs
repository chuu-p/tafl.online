use api::{ClientMessage, ServerMessage};
use dioxus::fullstack::{use_websocket, WebSocketOptions};
use dioxus::prelude::*;

use views::{GameView, GameVs, Home};
use ui::Navbar;

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(WebNavbar)]
    #[route("/")]
    Home {},
    #[route("/game/:id")]
    GameView { id: String },
    #[route("/game/:id/:side")]
    GameVs { id: String, side: String },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        PingProvider {
            Router::<Route> {}
        }
    }
}

#[allow(unused_mut)]
#[component]
fn PingProvider(children: Element) -> Element {
    let mut ping_ms = use_signal(|| 0u64);
    let mut flash = use_signal(|| false);
    let mut connected = use_signal(|| false);
    let mut logs = use_signal(|| Vec::<String>::new());
    use_context_provider(|| (ping_ms, flash, connected, logs));

    let mut socket = use_websocket(|| api::ws(WebSocketOptions::new()));

    use_future(move || async move {
        loop {
            sleep(2000).await;

            #[cfg(feature = "web")]
            let now = js_sys::Date::now() as u64;
            #[cfg(not(feature = "web"))]
            let now = 0;

            log_add(logs, &format!("[ping] send ts={now}"));

            if socket
                .send(ClientMessage::Ping {
                    client_time_ms: now,
                })
                .await
                .is_err()
            {
                log_add(logs, "[ping] send failed");
                connected.set(false);
                continue;
            }

            match socket.recv().await {
                Ok(ServerMessage::Pong {
                    client_time_ms, ..
                }) => {
                    let _ = client_time_ms;
                    #[cfg(feature = "web")]
                    {
                        let rtt = js_sys::Date::now() as u64 - client_time_ms;
                        log_add(logs, &format!("[ping] pong rtt={rtt}ms"));
                        ping_ms.set(rtt);
                        connected.set(true);
                        flash.set(true);
                        spawn(async move {
                            sleep(300).await;
                            flash.set(false);
                        });
                    }
                }
                Err(e) => {
                    log_add(logs, &format!("[ping] recv error: {e:?}"));
                    connected.set(false);
                }
                _ => {
                    log_add(logs, "[ping] unexpected message");
                    connected.set(false);
                }
            }
        }
    });

    rsx! { {children} }
}

fn log_add(mut logs: Signal<Vec<String>>, msg: &str) {
    #[cfg(feature = "web")]
    web_sys::console::log_1(&msg.into());
    logs.write().push(msg.to_string());
    if logs.read().len() > 50 {
        logs.write().remove(0);
    }
}

async fn sleep(ms: u32) {
    #[cfg(feature = "web")]
    gloo_timers::future::sleep(std::time::Duration::from_millis(ms as u64)).await;
    #[cfg(not(feature = "web"))]
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
}

#[component]
fn WebNavbar() -> Element {
    let (ping_ms, flash, connected, _logs) = use_context::<(Signal<u64>, Signal<bool>, Signal<bool>, Signal<Vec<String>>)>();
    let ping_color = if flash() { "#ffffff" } else { "#f0f6f0" };
    let ping_shadow = if flash() { "0 0 6px #f0f6f0" } else { "none" };

    rsx! {
        Navbar {
            Link {
                to: Route::Home {},
                "tafl.online"
            }
        }
        div {
            style: "position: fixed; top: 12px; right: 16px; font-size: 0.85rem; z-index: 1000; transition: color 0.15s, text-shadow 0.15s;",
            style: "color: {ping_color}; text-shadow: {ping_shadow};",
            if connected() { "ping: {ping_ms}ms" } else { "disconnected" }
        }
        Outlet::<Route> {}
    }
}
