use bollard::Docker;
use bollard::container::{CreateContainerOptions, Config as DockerRunConfig, HostConfig, StartContainerOptions};
use chrono::prelude::*;
use serde_derive::Deserialize;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use std::str;

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
  backup_command: Option<String>,
}

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), String> {
  let args: Vec<String> = env::args().collect();
  
  if args.len() < 2 {
    panic!("You need to provide the config.toml file as first parameter");
  }

  let config = get_config(&args[1])?;

  let docker = Docker::connect_with_unix_defaults().map_err(stringify)?;

  for backup_config in config.projects {
    backup_docker_compose_service(&docker, config.backup_directory.as_str(), backup_config).await?;
  }

  Ok(())
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

async fn backup_docker_compose_service(docker: &Docker, backup_directory: &str, backup_config: BackupConfig) -> Result<(), String> {
  let binds = format!("{}:/backup", backup_directory);

  let container_id = extract_container_id(&backup_config)?;

  let utc: DateTime<Utc> = Utc::now();

  let container_name = format!("{}-backup", &backup_config.service);

  let options = Some(CreateContainerOptions{
    name: container_name.as_str(),
  });

  let backup_command = match backup_config.backup_command {
    Some(command) => format!("cd {} && {}", &backup_config.path, &command),
    None => format!("cd {} && tar cf /backup/{}_{}.tar .", &backup_config.path, &backup_config.service, &utc.to_rfc3339()),
  };

  let mut host_config = HostConfig::default();
  host_config.binds = Some(vec![binds.as_str()]);
  host_config.volumes_from = Some(vec![container_id.as_str()]);
  host_config.auto_remove = Some(true);

  let config = DockerRunConfig {
    image: Some("ubuntu"),
    cmd: Some(vec![
      "bash",
      "-c",
      backup_command.as_str(),
    ]),
    host_config: Some(host_config),
    ..Default::default()
  };

  docker.create_container(options, config).await.map_err(stringify)?;

  docker.start_container(&container_name, None::<StartContainerOptions<String>>).await.map_err(stringify)?;

  Ok(())
}

fn extract_container_id(backup_config: &BackupConfig) -> Result<String, String> {
  let output = Command::new("docker-compose")
      .current_dir(&backup_config.docker_compose)
      .arg("ps")
      .arg("--quiet")
      .arg(&backup_config.service)
      .output()
      .map_err(stringify)?;

  let container_id = str::from_utf8(&output.stdout).map_err(stringify)?.trim();

  match container_id {
    "" => Err(format!("Unable to extract a container id for service: {} in {}/docker-compose.yml", backup_config.service, backup_config.path)),
    id => Ok(id.into())
  }
}