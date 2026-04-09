# Deploy Docker Image to a Remote Host

This guide shows how to export the `ichat-web` Docker image, transfer it to a remote server with `scp`, and load it there.

Replace the placeholders below with your own values:

- `<IMAGE>`: Docker image name, for example `ghcr.io/example/ichat-web:latest`
- `<ARCHIVE_PATH>`: local tar file path, for example `/tmp/ichat-web.tar`
- `<SSH_KEY_PATH>`: SSH private key path, for example `~/.ssh/deploy_key`
- `<SSH_USER>`: remote SSH username
- `<SSH_HOST>`: remote SSH host or IP
- `<REMOTE_DIR>`: remote directory to upload into

## 1. Export the image

```bash
docker save <IMAGE> -o <ARCHIVE_PATH>
```

Example:

```bash
docker save ghcr.io/example/ichat-web:latest -o /tmp/ichat-web.tar
```

## 2. Copy the image to the remote machine

```bash
scp -i <SSH_KEY_PATH> <ARCHIVE_PATH> <SSH_USER>@<SSH_HOST>:<REMOTE_DIR>/
```

Example:

```bash
scp -i ~/.ssh/deploy_key /tmp/ichat-web.tar <SSH_USER>@<SSH_HOST>:<REMOTE_DIR>/
```

## 3. Load the image on the remote machine

```bash
ssh -i <SSH_KEY_PATH> <SSH_USER>@<SSH_HOST> 'docker load -i <REMOTE_DIR>/ichat-web.tar'
```

If your archive file name is different, update the remote path to match it.

## 4. Run the container on the remote machine

```bash
ssh -i <SSH_KEY_PATH> <SSH_USER>@<SSH_HOST> '
docker run -d \
  --name ichat-web \
  --restart unless-stopped \
  -p 3000:80 \
  -v <REMOTE_DIR>/data:/data \
  -e API_KEY=<YOUR_API_KEY> \
  -e NEWAPI_AUTH_BASE=<YOUR_NEWAPI_AUTH_BASE> \
  -e API_BASE=<YOUR_API_BASE> \
  <IMAGE>
'
```

## 5. Verify the deployment

```bash
ssh -i <SSH_KEY_PATH> <SSH_USER>@<SSH_HOST> 'docker ps'
ssh -i <SSH_KEY_PATH> <SSH_USER>@<SSH_HOST> 'docker logs --tail 100 ichat-web'
```

## Optional: Use Docker Compose on the remote host

If you prefer Compose, create a `docker-compose.yml` on the remote machine using your own environment values, then run:

```bash
ssh -i <SSH_KEY_PATH> <SSH_USER>@<SSH_HOST> 'docker compose up -d'
```
