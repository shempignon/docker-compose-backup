use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub backup_directory: String,
    pub projects: Vec<BackupConfig>,
    pub image: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct BackupConfig {
    pub service: String,
    pub docker_compose: String,
    pub path: String,
    pub backup_command: Option<String>,
}
