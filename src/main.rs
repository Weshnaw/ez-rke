use std::{fs, io, path::PathBuf, time::Duration};

use clap::Parser;
use ez_rke::{app::App, event::EventHandler, log::init_logger};

/// Simple automation tool to configure a clustered RKE2 service
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Name of the configuration file
    #[arg(short, long, default_value = "./config.toml")]
    config: PathBuf,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    let event_handler = EventHandler::new(Duration::from_millis(250));
    init_logger(&event_handler);

    let app = App::new(
        event_handler,
        toml::from_str(&fs::read_to_string(args.config).expect("Unable to read config file"))
            .expect("Unable to parse config file"),
    );

    app.run().await
}
