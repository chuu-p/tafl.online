use crate::views::WsPing;
use dioxus::prelude::*;
use ui::{Echo, Hero};

#[component]
pub fn Home() -> Element {
    rsx! {
        // Echo {}
        WsPing {}
    }
}
