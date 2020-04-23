Docker Compose Backup
===

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
- {path_to_backup} is the path in the running service container you want to backup

It uses the [docker engine api](https://docs.docker.com/engine/api/), through [bollard](https://github.com/fussybeaver/bollard) rather than the docker client.

Usage
---

- Create a `config.toml` (name does not matter), there is an [example file](examples/config.toml)
- The `backup_directory` is the directory where the archives will be saved
- For each service you want to backup you will need to provide a ``[[projects]]` entry:
```toml
[[projects]]
service = "service"
docker_compose = "/path/to/project/containing/docker-compose-file"
path = "/path/to/backup/in/container"
```
- If you want to use another backup command you can specify it with
```toml
[[projects]]
...
backup_command = "zip -r archive.zip ."
```
- Backup your service(s) running: `docker-compose-backup config.toml`