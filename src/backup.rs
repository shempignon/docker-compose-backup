use super::config::BackupConfig;
use super::config::Config;
use super::stringify::stringify;
use bollard::container::{
    Config as DockerRunConfig, CreateContainerOptions, HostConfig, StartContainerOptions,
};
use bollard::image::{CreateImageOptions, SearchImagesOptions};
use bollard::Docker;
use chrono::prelude::*;
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
    pub async fn process(&self) -> Result<(), String> {
        let image = self.extract_image();
        let tag = self.extract_tag();

        if self.should_pull(image, tag).await? {
            self.pull_image(image, tag).await?;
        }

        for backup_config in &self.config.projects {
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

    async fn should_pull(&self, image: &str, tag: Option<&str>) -> Result<bool, String> {
        let term = Self::get_reference(image, tag);

        let search_options = SearchImagesOptions {
            term,
            ..Default::default()
        };

        let results = self
            .docker
            .search_images(search_options)
            .await
            .map_err(stringify)?;

        Ok(results.is_empty())
    }

    async fn pull_image(&self, from_image: &str, tag_option: Option<&str>) -> Result<(), String> {
        let mut options = CreateImageOptions {
            from_image,
            ..Default::default()
        };

        if let Some(tag) = tag_option {
            options.tag = tag;
        }

        use futures::stream::TryStreamExt;

        let mut stream = self.docker.create_image(Some(options), None, None);

        stream.try_next().await.map_err(stringify)?;

        Ok(())
    }

    async fn backup_project(
        &self,
        backup_config: &BackupConfig,
        image: &str,
        tag: Option<&str>,
    ) -> Result<(), String> {
        let binds = format!("{}:/backup", self.config.backup_directory);

        let container_id = &self.extract_container_id(&backup_config)?;

        let utc: DateTime<Utc> = Utc::now();

        let container_name = format!("{}-backup", &backup_config.service);

        let options = Some(CreateContainerOptions {
            name: container_name.as_str(),
        });

        let backup_command = match &backup_config.backup_command {
            Some(command) => format!("cd {} && {}", &backup_config.path, &command),
            None => format!(
                "cd {} && tar cf /backup/{}_{}.tar .",
                &backup_config.path,
                &backup_config.service,
                &utc.to_rfc3339()
            ),
        };

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
            .map_err(stringify)?;

        self.docker
            .start_container(&container_name, None::<StartContainerOptions<String>>)
            .await
            .map_err(stringify)?;

        Ok(())
    }

    fn extract_container_id(&self, backup_config: &BackupConfig) -> Result<String, String> {
        let output = Command::new("docker-compose")
            .current_dir(&backup_config.docker_compose)
            .arg("ps")
            .arg("--quiet")
            .arg(&backup_config.service)
            .output()
            .map_err(stringify)?;

        let container_id = str::from_utf8(&output.stdout).map_err(stringify)?.trim();

        match container_id {
            "" => Err(format!(
                "Unable to extract a container id for service: {} in {}/docker-compose.yml",
                &backup_config.service, &backup_config.path
            )),
            id => Ok(id.into()),
        }
    }
}
