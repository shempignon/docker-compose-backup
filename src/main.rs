#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

mod backup;
mod config;
mod report;

use backup::Backup;
use bollard::Docker;
use clap::{App, Arg};
use config::Config;
use report::report;
use std::fs::File;
use std::io::prelude::*;

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), ()> {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("config")
                .index(1)
                .required(true)
                .help("Sets the toml config file to use"),
        )
        .arg(
            Arg::with_name("verbosity")
                .short("v")
                .multiple(true)
                .help("Increase message verbosity"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .help("Silence all output"),
        )
        .get_matches();

    let verbose = matches.occurrences_of("verbosity") as usize;
    let quiet = matches.is_present("quiet");

    stderrlog::new()
        .module(module_path!())
        .quiet(quiet)
        .verbosity(verbose)
        .init()
        .unwrap();

    let config_file = matches.value_of("config").unwrap();

    let config = get_config(&config_file)?;

    let docker = Docker::connect_with_unix_defaults().map_err(report)?;

    let backup = Backup::new(&config, &docker);

    backup.process().await?;

    Ok(())
}

fn get_config(path: &str) -> Result<Config, ()> {
    let mut file = File::open(path).map_err(report)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(report)?;

    let config: Config = toml::from_str(&contents).map_err(report)?;

    Ok(config)
}
