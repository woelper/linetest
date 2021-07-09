use gumdrop::Options;
use linetest;

#[derive(Debug, Options)]
struct MyOptions {
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
    ping_interval: Option<i32>,

    // A `Vec` field will accumulate all values received from the command line.
    #[options(help = "Supply your own download urls")]
    download_urls: Vec<String>,

}


fn main() {
    let opts = MyOptions::parse_args_default_or_exit();

    let mut measurement = linetest::Measurement::default();

    let receiver = measurement.run_periodic().unwrap();
    loop {

        for dp in &receiver {
    
            println!("{}", dp);
        }
    }

}
