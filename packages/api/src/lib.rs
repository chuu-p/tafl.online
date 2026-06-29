//! This crate contains all shared fullstack server functions.
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
    let diff = target_time.duration_since(now).unwrap();
    println!(
        "The target timestamp is {} ms in the future.",
        diff.as_millis()
    );

    Ok(diff.as_millis().to_string())
}
