use clap::ArgMatches;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub brokers: String,
}

impl Config {
    pub fn new(args: &ArgMatches) -> Config {
        let file_path = args.value_of("config-file").unwrap();
        let content = fs::read_to_string(file_path).expect("Something went wrong reading the file");
        let config_from_file: Result<Config, _> = serde_json::from_str(&content);

        let brokers = args
            .value_of("brokers")
            .map(|v| v.to_string())
            .unwrap_or_else(|| config_from_file.expect("Failed to find conf file").brokers);
        Config { brokers }
    }
}
