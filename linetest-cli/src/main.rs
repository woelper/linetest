use std::time::Duration;

use gumdrop::Options;
use linetest::{self, Datapoint, Evaluation};
use std::io::{stdout};

use crossterm::style::{Color, Colors, Print, SetColors};
use crossterm::{
    cursor::{Hide, RestorePosition, SavePosition},
    execute,
    terminal::{Clear, ClearType},
    Result,
};

#[derive(Debug, Options)]
struct LinetestOptions {
    // // Contains "free" arguments -- those that are not options.
    // // If no `free` field is declared, free arguments will result in an error.
    // #[options(free)]
    // free: Vec<String>,

    // // Non-boolean fields will take a value from the command line.
    // // Wrapping the type in an `Option` is not necessary, but provides clarity.
    // #[options(help = "give a string argument")]
    // string: Option<String>,

    // A field can be any type that implements `FromStr`.
    // The optional `meta` attribute is displayed in `usage` text.
    #[options(help = "Time in seconds between pings")]
    ping_delay: Option<u64>,

    // A `Vec` field will accumulate all values received from the command line.
    #[options(help = "Supply your own download urls")]
    download_urls: Vec<String>,
}

/// Primitive function to draw the results
fn draw_ui(result: &linetest::MeasurementResult) -> Result<()> {
    execute!(
        stdout(),
        //SetColors(Colors::new(Green, Black)),
        Clear(ClearType::CurrentLine),
        SavePosition,
        Hide
    )?;


    let mut dp_ping: Option<&Datapoint> = None;
    let mut dp_dl: Option<&Datapoint> = None;


    for res in result {
        match res {
            Datapoint::Latency(_l, _t) => {
                dp_ping = Some(res);
            }
            Datapoint::ThroughputDown(_tp, _t) => {
                // dbg!("dn");
                dp_dl = Some(res);
            }
            _ => (),
        }
    }



    match dp_ping {
        Some(dp) => {
            execute!(
                stdout(),
                Print(format!("{}", dp)),
            )?;
        },
        None => {
            execute!(
                stdout(),
                Print("Please wait..."),
            )?;
        }
    }

    match dp_dl {
        Some(dp) => {
            execute!(
                stdout(),
                Print(format!("\n{}", dp)),
            )?;
        },
        None => {
            execute!(
                stdout(),
                Print("\nSpeed:\tPlease wait..."),
            )?;
        }
    }
    

    execute!(
        stdout(),
        RestorePosition
    )?;

    Ok(())
}

fn main() {
    std::env::set_var("RUST_LOG", "warning");
    // #[cfg(debug_assertions)]
    // std::env::set_var("RUST_LOG", "info");

    let _ = env_logger::try_init();

    let opts = LinetestOptions::parse_args_default_or_exit();

    let mut measurement = linetest::MeasurementBuilder::default();

    if !opts.download_urls.is_empty() {
        measurement.downloads_urls = opts.download_urls
    }

    if let Some(s) = opts.ping_delay {
        measurement.ping_delay = Duration::from_secs(s);
    }

    let receiver = measurement.run_until_receiver_drops().unwrap();
    let mut measurement_result = vec![];

    println!("[[[ Linetest ]]]");
    if let Some(log) = &measurement.logfile {
        println!("=> This session is recorded to {}", log.to_string_lossy());
    }

    loop {
        for dp in &receiver {
            measurement_result.push(dp);
            if let Some(log) = &measurement.logfile {
                // save each entry
                measurement_result.save(log).unwrap();
            }
            draw_ui(&measurement_result).unwrap();
        }
    }
}
