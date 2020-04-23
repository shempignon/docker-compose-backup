use serde_derive::Deserialize;
use std::env;
use std::fs::File;
use std::io::prelude::*;

#[derive(Deserialize, Debug)]
struct Config {
  backup_directory: String,
  projects: Vec<BackupConfig>
}

#[derive(Deserialize, Debug)]
struct BackupConfig {
  service: String,
  docker_compose: String,
  path: String,
}

fn main() {
  let args: Vec<String> = env::args().collect();
  
  if args.len() < 2 {
    panic!("You need to provide the config.toml file as first parameter");
  }

  let config = get_config(&args[1]).unwrap();

  println!("{:?}", config);
}

fn stringify<T>(err: T) -> String where T: ToString {
  err.to_string()
}

fn get_config(path: &str) -> Result<Config, String> {
  let mut file = File::open(path).map_err(stringify)?;
  let mut contents = String::new();
  file.read_to_string(&mut contents).map_err(stringify)?;

  let config: Config = toml::from_str(&contents).map_err(stringify)?;

  Ok(config)
}