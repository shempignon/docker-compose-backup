use serde_derive::Deserialize;

#[derive(Deserialize)]
struct Config {
  backup_directory: String,
  projects: Vec<BackupConfig>
}

#[derive(Deserialize)]
struct BackupConfig {
  service: String,
  docker_compose: String,
  path: String,
}

fn main() {
    println!("Hello, world!");
}
