# HomeFlix

A self-hosted media server with a Netflix-like interface for your personal media library.

## ⚠️ WARNING: Work in Progress

**This project is highly work in progress and was vibe coded.**

- ⚠️ **Not production ready** - This is experimental software with incomplete features
- 🎨 **Vibe coded** - Code quality and architecture decisions were made on the fly
- 🐛 **Expect bugs** - Many features are untested or partially implemented
- 📝 **Documentation may be outdated** - Code changes faster than docs
- 🔧 **Breaking changes** - API and behavior may change without notice
- 💥 **Use at your own risk** - Data loss, crashes, and unexpected behavior are possible

**If you're looking for a stable, production-ready media server, this is not it. This is a personal project in active development.**

---

## Components

| Component | Description | Status |
|-----------|-------------|--------|
| [web](./web) | SvelteKit web frontend | Active |
| server | Rust backend (Axum) | Coming soon |
| mobile | Flutter mobile/TV client | Coming soon |

## Quick Start with Docker

```bash
# Pull the web frontend image
docker pull ghcr.io/drmckay/homeflix/web:latest

# Run the container
docker run -d \
  -p 3000:3000 \
  -e VITE_API_URL=http://your-backend:3001 \
  ghcr.io/drmckay/homeflix/web:latest
```

## Development

See individual component READMEs for development instructions:
- [Web Frontend](./web/README.md)

## License

MIT
