use dioxus::prelude::*;

const ECHO_CSS: Asset = asset!("/assets/styling/echo.css");

/// Echo component that demonstrates fullstack server functions.
#[component]
pub fn Echo() -> Element {
    let mut response = use_signal(|| String::new());

    rsx! {
        document::Link { rel: "stylesheet", href: ECHO_CSS }
        div {
            id: "echo",
            h4 { "ServerFn Echo" }
            input {
                placeholder: "Type here to echo...",
                oninput:  move |_event| async move {
                    #[cfg(feature = "web")]
                    let now = js_sys::Date::now() as u64;
                    #[cfg(not(feature = "web"))]
                    let now = 0;

                    #[cfg(feature = "web")]
                    web_sys::console::log_1(&format!("pinging with now: {}", now).into());

                    let data = api::ping(now).await.unwrap();

                    response.set(data);
                },
            }

            if !response().is_empty() {
                p {
                    "Server echoed: "
                    i { "{response}" }
                }
            }
        }
    }
}
