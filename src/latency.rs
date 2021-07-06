
use pinger::{ping, PingResult};
use anyhow::Error;


pub fn do_ping(addr: &str) {

    let stream = ping(addr.to_string()).expect("Error pinging");
    for message in stream {
        match message {
            PingResult::Pong(duration, _) => println!("{:?}", duration),
            PingResult::Timeout(_) => println!("Timeout!"),
            // Unknown lines, just ignore.
            PingResult::Unknown(line) => ()
        }
    }

}