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
| [server](./server) | Rust backend (Axum) | Active |
| mobile | Flutter mobile/TV client | Coming soon |

## Quick Start with Docker

### Backend Server

```bash
# Build the backend server
docker build -f server/Dockerfile -t homeflix-server:latest .

# Run the backend
docker run -d \
  -p 3000:3000 \
  -v ./data:/data \
  -v /path/to/media:/media \
  -e MEDIA_DIR=/media \
  -e TMDB_API_KEY=your_tmdb_api_key \
  -e DATABASE_URL=sqlite:/data/data.db?mode=rwc \
  homeflix-server:latest
```

**Required Environment Variables:**
- `MEDIA_DIR` - Path to your media library (mount as volume)
- `TMDB_API_KEY` - Get your API key from [TMDB](https://www.themoviedb.org/settings/api)

**Optional Environment Variables:**
- `DATABASE_URL` - SQLite connection string (default: `sqlite:data.db?mode=rwc`)
- `PORT` - Server port (default: `3000`)
- `SCAN_INTERVAL_SECS` - Background scan interval in seconds (default: `3600`)
- `RUST_LOG` - Log level: `error`, `warn`, `info`, `debug`, `trace` (default: `info`)

### Web Frontend

```bash
# Pull the web frontend image
docker pull ghcr.io/drmckay/homeflix-web:latest

# Run the container (point to your backend)
docker run -d \
  -p 3001:3000 \
  -e PUBLIC_API_URL=http://your-backend-host:3000 \
  ghcr.io/drmckay/homeflix-web:latest
```

**Note:** The `PUBLIC_API_URL` can be set at runtime, allowing the same Docker image to work with different backend URLs without rebuilding.

### Docker Compose (Full Stack)

Create a `docker-compose.yml` file:

```yaml
version: '3.8'

services:
  server:
    build:
      context: .
      dockerfile: ./server/Dockerfile
    ports:
      - "3000:3000"
    volumes:
      - ./data:/data
      - /path/to/media:/media  # Change to your media directory
    environment:
      - DATABASE_URL=sqlite:/data/data.db?mode=rwc
      - MEDIA_DIR=/media
      - TMDB_API_KEY=your_tmdb_api_key  # Get from https://www.themoviedb.org/settings/api
      - PORT=3000
      - RUST_LOG=info
      - SCAN_INTERVAL_SECS=3600
      # Optional: Whisper configuration
      # - WHISPER_MODEL_PATH=/app/models/ggml-small.bin
      # - WHISPER_CLI_PATH=whisper-cli
      # Optional: Ollama configuration (if running separately)
      # - OLLAMA_URL=http://ollama:11434
      # - OLLAMA_MODEL=llama3.2
    # Optional: GPU support for Whisper
    # deploy:
    #   resources:
    #     reservations:
    #       devices:
    #         - driver: nvidia
    #           count: 1
    #           capabilities: [gpu, video, compute, utility]
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 5s

  web:
    image: ghcr.io/drmckay/homeflix-web:latest
    # Or build locally:
    # build:
    #   context: ./web
    #   dockerfile: Dockerfile
    ports:
      - "3001:3000"
    environment:
      - PUBLIC_API_URL=http://server:3000
    depends_on:
      - server
```

Then start everything:

```bash
docker-compose up -d
```

The backend will be available at `http://localhost:3000` and the web frontend at `http://localhost:3001`.

## API Endpoints

The backend provides a REST API at `/v2/*`:

### Media
- `GET /v2/media` - List grouped library (recent, continue watching, categories)
- `GET /v2/media/recent` - List recently added media
- `GET /v2/media/all` - List all media
- `GET /v2/media/:id` - Get media details
- `GET /v2/media/:id/tracks` - Get audio/subtitle tracks
- `GET /v2/media/:id/credits` - Get cast and crew credits
- `POST /v2/media/:id/identify` - Manually identify media

### Series
- `GET /v2/series` - List all TV series
- `GET /v2/series/:id` - Get series details

### Collections
- `GET /v2/collections` - List all collections
- `GET /v2/collections/:id` - Get collection details

### Streaming
- `GET /v2/stream/:id` - Stream video (direct MP4)
- `GET /v2/stream/web/:id` - Stream video (web player with transcoding)
- `GET /v2/stream/diagnostic/:id` - Get streaming diagnostic info
- `GET /v2/thumbnail/:id` - Generate thumbnail
- `GET /v2/subtitles/:media_id/:index` - Get subtitle file (WebVTT)

### Progress Tracking
- `GET /v2/progress/:id` - Get watch progress
- `POST /v2/progress/:id` - Update watch progress
- `POST /v2/progress/:id/watched` - Mark as watched
- `DELETE /v2/progress/:id/watched` - Mark as unwatched

### Search
- `GET /v2/search` - Search media
- `GET /v2/search/series` - Search TV series

### Subtitle Generation
- `GET /v2/subtitles/capabilities` - Check Whisper/Ollama availability
- `GET /v2/subtitles/active` - Get active subtitle generation jobs
- `POST /v2/subtitles/:media_id/generate` - Generate subtitle
- `GET /v2/subtitles/jobs/:job_id` - Get job status
- `DELETE /v2/subtitles/jobs/:job_id` - Cancel job
- `POST /v2/subtitles/batch/generate` - Batch generate subtitles
- `GET /v2/subtitles/batch/jobs/:job_id` - Get batch job status
- `DELETE /v2/subtitles/batch/jobs/:job_id` - Cancel batch job

### Utilities
- `GET /health` - Health check endpoint
- `POST /v2/scan` - Trigger manual library scan
- `GET /v2/images/proxy` - Proxy TMDB images (CORS bypass)

## Features

### Backend
- 🎬 Automatic media scanning and identification
- 🎥 Direct video streaming with FFmpeg transcoding
- 🎤 Multi-audio track support
- 📝 Automatic subtitle generation (Whisper + Ollama)
- 📊 Watch progress tracking
- 🎭 TMDB metadata integration
- 📺 TV series support with season/episode detection
- 🎞️ Collection presets for franchise timelines (Star Trek, Stargate, MCU, etc.)
- 🔄 Background scanning with configurable intervals
- 💾 SQLite database for metadata storage
- 🖼️ Image caching for TMDB posters

### Frontend
- 🎨 Netflix-like interface
- 📱 Responsive design
- 🌍 Internationalization (English, Hungarian)
- 🔍 Search functionality
- 📺 Video player with subtitle support
- 📊 Watch progress visualization

## Development

See individual component READMEs for development instructions:
- [Backend Server](./server/README.md)
- [Web Frontend](./web/README.md)

## License

MIT
