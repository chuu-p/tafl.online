//! This crate contains all shared fullstack server functions.
use dioxus::fullstack::{WebSocketOptions, Websocket};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Echo the user input on the server.
#[post("/api/echo")]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    Ok(input)
}

#[post("/api/ping")]
pub async fn ping(input: u64) -> Result<String, ServerFnError> {
    let now = SystemTime::now();
    let target_time = UNIX_EPOCH + Duration::from_millis(input);
    let diff = target_time
        .duration_since(now)
        .unwrap_or_else(|e| e.duration());
    println!("The target timestamp is {} ms away.", diff.as_millis());

    Ok(diff.as_millis().to_string())
}

#[get("/api/ping_ws")]
pub async fn ping_ws(options: WebSocketOptions) -> Result<Websocket<u64, String>> {
    Ok(options.on_upgrade(move |mut socket| async move {
        while let Ok(msg) = socket.recv().await {
            let now = SystemTime::now();
            let target = UNIX_EPOCH + Duration::from_millis(msg);
            let diff = target.duration_since(now).unwrap_or_else(|e| e.duration());
            println!(
                "The WS target timestamp is {} micros away.",
                diff.as_micros()
            );
            _ = socket.send(diff.as_millis().to_string()).await;
        }
    }))
}
