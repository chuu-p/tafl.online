use dioxus::prelude::*;
use ui::{Echo, Hero};
use crate::views::WsPing;

#[component]
pub fn Home() -> Element {
    rsx! {
        Hero {}
        Echo {}
        WsPing {}
    }
}
