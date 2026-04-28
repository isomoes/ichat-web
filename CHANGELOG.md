# Changelog

All notable changes to this project will be documented in this file.

`v0.1.0` is the first release published after resetting project versioning.

## [0.1.1] - 2026-04-28

### Added

- Added deployment documentation for packaged releases. (@isomoes)

### Changed

- Updated the Docker Compose deployment configuration. (@isomoes)
- Switched China web search requests to Bing. (@isomoes)
- Updated copyright ownership to isomoes. (@isomoes)

### Fixed

- Extracted PDF text before upload so PDF attachments are sent as text content. (@isomoes)

## [0.1.0] - 2026-04-09

Initial `ichat` release.

### Changed

- Reset project package versioning to `0.1.0`. (@isomoes)
- Renamed the product branding from llumen to ichat. (@isomoes)
- Proxy frontend development API requests through Vite for local development. (@isomoes)
- Added this changelog to track releases going forward. (@isomoes)

### Added

- Added an external authentication registration flow. (@isomoes)

### Fixed

- Isolated chat history across user sessions. (@isomoes)
- Supported browser plugin scrolling inside chat. (@isomoes)
- Limited upstream attachment size handling. (@isomoes)
- Kept the chat view pinned to the bottom while streaming responses. (@isomoes)
- Persisted and reused per-user NewAPI keys. (@isomoes)
- Suppressed the unauthorized toast during logout. (@isomoes)
- Fixed the email verification flow during registration. (@isomoes)
- Used per-user NewAPI model lists. (@isomoes)

### Removed

- Removed outdated GitHub Actions workflow files. (@isomoes)
- Removed the static document. (@isomoes)
