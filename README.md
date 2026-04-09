<div align="center">
  <img src="frontend/static/web-app-manifest-512x512.png" alt="ichat Logo" width="200" height="auto" />

  # ichat

  [![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](https://www.mozilla.org/en-US/MPL/2.0/)
  [![Nightly Docker](https://github.com/isomoes/ichat-web/actions/workflows/docker-nightly.yml/badge.svg)](https://github.com/isomoes/ichat-web/actions/workflows/docker-nightly.yml)
  [![status check](https://github.com/isomoes/ichat-web/actions/workflows/check.yml/badge.svg)](https://github.com/isomoes/ichat-web/actions/workflows/check.yml)
  ![MSRV](https://img.shields.io/static/v1?label=MSRV&message=1.89&color=orange&logo=rust)
</div>

<div align="center">
  
  [![en](https://img.shields.io/badge/lang-en-green)](./README.md)
  [![zh-tw](https://img.shields.io/badge/lang-zh--TW-green)](./README_zh-TW.md)
  [![zh-cn](https://img.shields.io/badge/lang-zh--CN-green)](./README_zh-CN.md)
</div>

---

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
| Universal API | Any OpenAI-compatible provider (OpenRouter, local models, etc.) |
| Minimal Footprint | ~17MB binary, <128MB RAM usage |

[![Video preview](https://isomoes.github.io/ichat-web/img/theme/ichat.webp)](https://github.com/user-attachments/assets/4d46e649-bd33-4850-af2b-59527cc11618)

## Quickstart

> **Default Login:** `admin` / `P@88w0rd`

### Docker (30-second setup)

```bash
docker run -it --rm \
  -e API_KEY="<YOUR_OPENROUTER_KEY>" \
  -p 80:80 \
  -v "$(pwd)/data:/data" \
  ghcr.io/isomoes/ichat-web:latest
```

That's it. No config files. No Python dependencies.

**Want bleeding edge?** Use `ghcr.io/isomoes/ichat-web:nightly`

See [docker sample](https://isomoes.github.io/ichat-web/user/docker) for docker-compose examples.

### Native Binaries

Download from [Releases](https://github.com/isomoes/ichat-web/releases) for Windows/Linux, including arm build.

## Building from Source

See [BUILD.md](./BUILD.md) for detailed build instructions.

## Configuration (Optional)

| Variable | Description | Default |
| :--- | :--- | :--- |
| `API_KEY` | OpenRouter/provider key | *required* |
| `API_BASE` | Custom endpoint | `https://openrouter.ai/api` |
| `DATA_PATH` | Storage directory | `.` |
| `BIND_ADDR` | Network Socket | `0.0.0.0:80` |

## Documentation

https://isomoes.github.io/ichat-web/

<div align="center">
  Built with :heart: by pinkfuwa. Keep it simple, keep it fast.
</div>
