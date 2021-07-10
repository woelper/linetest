use anyhow::Error;
use log::debug;
use pinger::{ping, PingResult};
use std::time::Duration;

pub fn ping_callback<F: FnMut(Option<Duration>)>(addr: &str, mut callback: F) -> Result<(), Error> {
    let stream = ping(addr.to_string())?;
    for message in stream {
        debug!("Ping msg {}", message);
        match message {
            PingResult::Pong(duration, _) => callback(Some(duration)),
            PingResult::Timeout(_) => callback(None),
            // Unknown lines, just ignore.
            PingResult::Unknown(_line) => (),
        }
        return Ok(());
    }
    Ok(())
}
