use dioxus::prelude::*;

#[component]
pub fn DebugConsole() -> Element {
    let logs = use_context::<Signal<Vec<String>>>();

    rsx! {
        div {
            style: "margin-top: 24px; padding: 8px; border-top: 1px solid #f0f6f0; font-size: 0.75rem; font-family: monospace; white-space: pre-wrap; word-break: break-all; max-height: 300px; overflow-y: auto;",
            for line in logs().iter().rev().take(20) {
                div { "{line}" }
            }
        }
    }
}
