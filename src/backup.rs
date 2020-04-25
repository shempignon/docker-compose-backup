use super::config::BackupConfig;
use super::config::Config;
use super::report::report;
use bollard::container::{
    Config as DockerRunConfig, CreateContainerOptions, HostConfig, InspectContainerOptions,
    StartContainerOptions,
};
use bollard::image::{CreateImageOptions, SearchImagesOptions};
use bollard::Docker;
use chrono::prelude::*;
use std::collections::HashMap;
use std::default::Default;
use std::process::Command;
use std::str;

pub struct Backup<'a> {
    config: &'a Config,
    docker: &'a Docker,
}

impl<'a> Backup<'a> {
    pub fn new(config: &'a Config, docker: &'a Docker) -> Self {
        Backup { config, docker }
    }
}

impl Backup<'_> {
    pub async fn process(&self) -> Result<(), ()> {
        let image = self.extract_image();
        let tag = self.extract_tag();

        if self.should_pull(image, tag).await? {
            info!("Pulling {}", Self::get_reference(image, tag));
            self.pull_image(image, tag).await?;
        }

        for backup_config in &self.config.projects {
            info!("Backing up service {}", backup_config.service);
            self.backup_project(backup_config, image, tag).await?;
        }

        Ok(())
    }

    fn extract_image(&self) -> &str {
        match &self.config.image {
            None => "ubuntu",
            Some(image_tag) => {
                let fragments: Vec<&str> = image_tag.split(':').collect();

                fragments.first().unwrap()
            }
        }
    }

    fn extract_tag(&self) -> Option<&str> {
        match &self.config.image {
            None => None,
            Some(image_tag) => {
                let fragments: Vec<&str> = image_tag.split(':').collect();

                if let Some(tag) = fragments.get(1) {
                    return Some(*tag);
                }

                None
            }
        }
    }

    fn get_reference(image: &str, tag_option: Option<&str>) -> String {
        let tag = tag_option.map_or("".into(), |tag| format!(":{}", tag));

        format!("{}{}", image, tag)
    }

    async fn should_pull(&self, image: &str, tag: Option<&str>) -> Result<bool, ()> {
        let term = Self::get_reference(image, tag);

        let search_options = SearchImagesOptions {
            term,
            ..Default::default()
        };

        let results = self
            .docker
            .search_images(search_options)
            .await
            .map_err(report)?;

        Ok(results.is_empty())
    }

    async fn pull_image(&self, from_image: &str, tag_option: Option<&str>) -> Result<(), ()> {
        let mut options = CreateImageOptions {
            from_image,
            ..Default::default()
        };

        if let Some(tag) = tag_option {
            options.tag = tag;
        }

        use futures::stream::TryStreamExt;

        let mut stream = self.docker.create_image(Some(options), None, None);

        stream.try_next().await.map_err(report)?;

        Ok(())
    }

    async fn backup_project(
        &self,
        backup_config: &BackupConfig,
        image: &str,
        tag: Option<&str>,
    ) -> Result<(), ()> {
        let binds = format!("{}:/backup", self.config.backup_directory);

        let container_id = &self.extract_container_id(&backup_config).map_err(report)?;

        let container_name = format!("{}-backup", &container_id);

        let options = Some(CreateContainerOptions {
            name: container_name.as_str(),
        });

        let mounts_destinations = self.retrieve_mount_destinations(&container_id).await?;

        let backup_command = self.build_backup_command(backup_config, mounts_destinations);

        let mut host_config = HostConfig::default();
        host_config.binds = Some(vec![binds.as_str()]);
        host_config.volumes_from = Some(vec![container_id.as_str()]);
        host_config.auto_remove = Some(true);

        let reference = Self::get_reference(image, tag);

        let config = DockerRunConfig {
            image: Some(reference.as_str()),
            cmd: Some(vec!["bash", "-c", backup_command.as_str()]),
            host_config: Some(host_config),
            ..Default::default()
        };

        self.docker
            .create_container(options, config)
            .await
            .map_err(report)?;

        self.docker
            .start_container(&container_name, None::<StartContainerOptions<String>>)
            .await
            .map_err(report)?;

        debug!("Image reference used {}", reference);
        debug!("Backup command used {}", backup_command);
        info!(
            "Service {} backup complete, available in {}",
            backup_config.service, self.config.backup_directory
        );

        Ok(())
    }

    fn extract_container_id(&self, backup_config: &BackupConfig) -> Result<String, String> {
        let output = Command::new("docker-compose")
            .current_dir(&backup_config.docker_compose)
            .arg("ps")
            .arg("--quiet")
            .arg(&backup_config.service)
            .output()
            .map_err(|err| err.to_string())?;

        let container_ids = str::from_utf8(&output.stdout)
            .map_err(|err| err.to_string())?
            .trim();

        let fragments: Vec<&str> = container_ids.split('\n').collect();

        match fragments.first() {
            None => Err(format!(
                "Unable to extract a container id for service: {} in {}/docker-compose.yml",
                &backup_config.service, &backup_config.docker_compose
            )),
            Some(id) => Ok((*id).into()),
        }
    }

    async fn retrieve_mount_destinations(
        &self,
        container_id: &str,
    ) -> Result<HashMap<String, String>, ()> {
        let options = Some(InspectContainerOptions { size: false });

        let container = self
            .docker
            .inspect_container(container_id, options)
            .await
            .map_err(report)?;

        let mut mount_destinations = HashMap::new();

        for (index, mount) in container.mounts.into_iter().enumerate() {
            let name = mount.name.clone().unwrap_or_else(|| index.to_string());

            mount_destinations.insert(name, mount.destination);
        }

        Ok(mount_destinations)
    }

    fn build_backup_command(
        &self,
        backup_config: &BackupConfig,
        mount_destinations: HashMap<String, String>,
    ) -> String {
        let utc: DateTime<Utc> = Utc::now();

        let commands: Vec<String> = mount_destinations
            .iter()
            .map(|(name, destination)| {
                let backup_command = backup_config.backup_command.clone().unwrap_or(format!(
                    "tar cf /backup/{}_{}_{}.tar .",
                    backup_config.service,
                    &name,
                    &utc.to_rfc3339()
                ));

                format!("cd {} && {}", destination, backup_command)
            })
            .collect();

        commands.join(" && ")
    }
}
