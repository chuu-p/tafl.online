use dioxus::prelude::*;

#[component]
pub fn Crown(
    style: Option<String>,
    size: u32,
    fill: String,
    stroke: String,
    stroke_width: f64,
) -> Element {
    rsx! {
        svg {
            style: style.unwrap_or_default(),
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill,
            stroke,
            stroke_width: "{stroke_width}",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M2 4l3 12h14l3-12-6 7-4-7-4 7-6-7z" }
        }
    }
}

#[component]
pub fn Stone(
    style: Option<String>,
    size: u32,
    fill: String,
    stroke: String,
    stroke_width: f64,
) -> Element {
    rsx! {
        svg {
            style: style.unwrap_or_default(),
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill,
            stroke,
            stroke_width: "{stroke_width}",
            circle { cx: "12", cy: "12", r: "9" }
        }
    }
}

#[component]
pub fn X(style: Option<String>, size: u32, stroke: String, stroke_width: u32) -> Element {
    let sw = stroke_width.to_string();
    rsx! {
        svg {
            style: style.unwrap_or_default(),
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke,
            stroke_width: "{sw}",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            line { x1: "18", y1: "6", x2: "6", y2: "18" }
            line { x1: "6", y1: "6", x2: "18", y2: "18" }
        }
    }
}
