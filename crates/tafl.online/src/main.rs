#[cfg(feature = "server")]
mod auth;

use dioxus::prelude::*;

#[derive(Debug, Clone, Routable, PartialEq)]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
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
            let config = ServeConfig::default();

            let app = axum::Router::new()
                .route("/api/auth/login", axum::routing::get(auth::login_handler))
                .route("/api/auth/callback", axum::routing::get(auth::callback_handler))
                .route("/api/auth/me", axum::routing::get(auth::me_handler))
                .route("/api/auth/logout", axum::routing::get(auth::logout_handler))
                .layer(tower_cookies::CookieManagerLayer::new())
                .layer(axum::extract::Extension(auth_state))
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

    rsx! {
        nav { class: "navbar",
            div { class: "navbar-left",
                Link { to: Route::Home {}, class: "brand", "tafl.online" }
                a { href: "#", class: "nav-link play-link", "Play" }
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
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    rsx! {
        main { class: "home",
            div { class: "lobby",
                h2 { "Open Games" }
                table { class: "lobby-table",
                    thead {
                        tr {
                            th { "Name" }
                            th { "Time" }
                            th { "Mode" }
                        }
                    }
                    tbody {
                        tr {
                            td { "—" }
                            td { colspan: "2", class: "empty", "No games waiting. Click Play to create one." }
                        }
                    }
                }
            }
        }
    }
}
