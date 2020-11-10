use clap::{App as ClapApp, Arg};
use log::{error, info};

use crate::config::Config;
use std::process;

mod app;
mod config;
mod handlers;
mod kafka;
mod model;
mod offsets_consumer;
mod ui;
mod utils;

fn main() {
    // get args
    let matches = ClapApp::new("kafui")
        .name("kafui")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("Fetch and print the kafka cluster metadata")
        .arg(
            Arg::with_name("brokers")
                .short("b")
                .long("brokers")
                .help("Broker list in kafka format")
                .conflicts_with("config-file")
                .takes_value(true)
                .empty_values(false),
        )
        .arg(
            Arg::with_name("config-file")
                .short("c")
                .long("config")
                .help("path to a config file")
                .takes_value(true)
                .default_value("config.json"),
        )
        .get_matches();

    env_logger::init();
    info!("Starting application");

    let config = Config::new(&matches);
    if let Err(e) = app::run(config) {
        error!("application failed with error {}", e);
        process::exit(1);
    }
}
