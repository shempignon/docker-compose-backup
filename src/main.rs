#[macro_use]
extern crate clap;

mod backup;
mod config;
mod stringify;

use backup::Backup;
use bollard::Docker;
use clap::App;
use config::Config;
use std::fs::File;
use std::io::prelude::*;
use stringify::stringify;

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), String> {
    let yaml = load_yaml!("cli.yml");

    let matches = App::from_yaml(yaml).get_matches();

    let config_file = matches.value_of("config").unwrap();

    let config = get_config(&config_file)?;

    let docker = Docker::connect_with_unix_defaults().map_err(stringify)?;

    let backup = Backup::new(&config, &docker);

    backup.process().await?;

    Ok(())
}

fn get_config(path: &str) -> Result<Config, String> {
    let mut file = File::open(path).map_err(stringify)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(stringify)?;

    let config: Config = toml::from_str(&contents).map_err(stringify)?;

    Ok(config)
}
