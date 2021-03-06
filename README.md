Docker Compose Backup
===

![ci](https://github.com/shempignon/docker-compose-backup/workflows/ci/badge.svg)

Backup [docker-compose](https://github.com/docker/compose) services through data containers

Motivation
---

I needed something more robust and scalable than bash scripts to backup some services I run with docker-compose.

This binary is a glorified wrapper for:

``` bash
docker run --rm --volumes-from=$(docker-compose ps --quiet {service}) --volume {backup_directory}:/backup ubuntu bash -c "cd {path_to_backup} && tar cvf /backup/{service}_$(date +%F).tar ."
```

Where:
- {service} is the name of the service you want to backup
- {backup_directory} is where you want to store your backup
- {path_to_backup} is the path in the running service container you want to 

Except that we don't require {path_to_backup} since we can infer its value by inspecting mounts on the container we want to backup

It uses the [docker engine api](https://docs.docker.com/engine/api/), through [bollard](https://github.com/fussybeaver/bollard) rather than the docker client.

Usage
---

- Create a `config.toml` (name does not matter), there is an [example file](examples/config.toml)
- The `backup_directory` is the directory where the archives will be saved
- For each service you want to backup you will need to provide a `[[projects]]` entry:
```toml
[[projects]]
service = "service"
docker_compose = "/path/to/project/containing/docker-compose-file"
```
- [Ubuntu](https://hub.docker.com/_/ubuntu) is the default image used for the data container, you can change it in the top level configuration:
```toml
image = "busybox:1"
```
- If you want to use another backup command you can specify it with
```toml
[[projects]]
...
backup_command = "zip -r archive.zip ."
```
- Backup your service(s) running: `docker-compose-backup config.toml`