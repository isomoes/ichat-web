# ichat

## Why ichat?

**Most self-hosted interfaces are built for servers, not devices.** They're powerful but demand heavy resources and hours of configuration.

ichat carves out a different space: **privacy without the complexity**. You get the features you actually need, optimized for modest hardware: Raspberry Pi/old laptops/minimal VPS while keeping many features of commercial products.

|  | Privacy | Power | Setup |
| :--- | :--- | :--- | :--- |
| **Commercial** (ChatGPT) | ❌ Cloud-only | ✅ High | ✅ Zero-config |
| **Typical Self-Host** (Open WebUI) | ✅ Local | ✅ High | ❌ Config hell |
| **ichat** | ✅ Local | ⚖️ Just enough | ✅ Zero-config |

## Features

| Feature | What You Get |
| :--- | :--- |
| Speed | Sub-second cold starts, real-time token streaming |
| Chat Modes | Normal, Web Search, & Deep Research with autonomous agents |
| Rich Media | PDF uploads, LaTeX rendering, image generation |
| Universal API | Any OpenAI-compatible provider (OpenRouter, local models, NewAPI-backed setups, etc.) |
| NewAPI Backend Auth | Login, registration, session reuse, and per-user model access through a NewAPI backend |
| NewAPI Pay API | Forward account top-up and payment requests through the same NewAPI session |
| Minimal Footprint | ~17MB binary, <128MB RAM usage |


## Quickstart

> **Default Login:** `admin` / `P@88w0rd`

### Docker (30-second setup)

```bash
docker run -it --rm \
  -e API_KEY="<YOUR_API_KEY>" \
  -e NEWAPI_AUTH_BASE="https://aiapi.xjh2000.top/" \
  -e API_BASE="https://aiapi.xjh2000.top" \
  -p 3000:80 \
  -v "$(pwd)/data:/data" \
  ghcr.io/isomoes/ichat-web:latest
```

This setup is ready for a NewAPI-backed deployment out of the box. Just set `API_KEY` and adjust the NewAPI URLs if you use a different backend.

That's it. No config files. No Python dependencies.

### Docker Compose

```yaml
services:
  ichat:
    image: ghcr.io/isomoes/ichat-web:latest
    restart: on-failure:3
    environment:
      - "API_KEY=<YOUR_API_KEY>"
      - "NEWAPI_AUTH_BASE=https://aiapi.xjh2000.top/"
      - "API_BASE=https://aiapi.xjh2000.top"
    volumes:
      - "./data:/data"
    ports:
      - "3000:80"
```

Then run:

```bash
docker compose up -d
```

Open `http://localhost:3000` and sign in with the default account: `admin` / `P@88w0rd`.

See [docker sample](./docker-compose.yml) for more docker-compose examples.


## Building from Source

See [BUILD.md](./BUILD.md) for detailed build instructions.

## NewAPI Integration

ichat has first-class support for using NewAPI as the backend auth and account system while keeping the browser pointed at ichat only.

When `NEWAPI_AUTH_BASE` is set, ichat can:

- use NewAPI for login and registration
- reuse each user's NewAPI session server-side
- load per-user model lists from NewAPI
- reuse each user's linked NewAPI API key
- forward balance top-up and payment requests to NewAPI's pay API

Configure the backend with:

```bash
export NEWAPI_AUTH_BASE="https://newapi.example.com"
export NEWAPI_AUTH_BEARER="optional-service-token"
export NEWAPI_AUTH_USER_HEADER="New-Api-User"
```

This lets ichat proxy auth and account actions server-to-server without exposing the NewAPI backend directly to the browser.

## Configuration (Optional)

| Variable | Description | Default |
| :--- | :--- | :--- |
| `API_KEY` | OpenAI-compatible provider key | *required* |
| `API_BASE` | Custom endpoint | `https://openrouter.ai/api` |
| `DATA_PATH` | Storage directory | `.` |
| `BIND_ADDR` | Network Socket | `0.0.0.0:80` |
| `NEWAPI_AUTH_BASE` | Optional NewAPI auth server base URL | unset |
| `NEWAPI_AUTH_BEARER` | Optional bearer token for NewAPI auth requests | unset |
| `NEWAPI_AUTH_USER_HEADER` | Optional header name used to forward the NewAPI user identity | `New-Api-User` |

## Acknowledgement

ichat builds on the earlier `llumen` project by pinkfuwa:

https://github.com/pinkfuwa/llumen
