# Homeflix Backend

Rust backend for self-hosted media server (Netflix-like) with automatic media scanning, streaming, and subtitle generation.

---


## Prerequisites

- **Rust** 1.92.0+ (for local development)
- **Docker** & **Docker Compose** (for containerized deployment)
- **NVIDIA GPU** with CUDA support (optional, for Whisper/Ollama subtitle generation)
- **FFmpeg** (included in Docker image)

## Quick Start

### Local Development

1. **Set required environment variables:**
   ```bash
   export MEDIA_DIR=/path/to/your/media/library
   export TMDB_API_KEY=your_tmdb_api_key
   ```

2. **Run the server:**
   ```bash
   cd server
   cargo run
   ```

   The server will start on `http://localhost:3000`

### Docker Deployment

1. **Build and start with Docker Compose:**
   ```bash
   docker-compose up -d server
   ```

2. **Or build manually:**
   ```bash
   docker build -f server/Dockerfile -t homeflix-server:latest .
   docker run -d \
     -p 3000:3000 \
     -v ./data:/data \
     -v /path/to/media:/media \
     -e MEDIA_DIR=/media \
     -e TMDB_API_KEY=your_key \
     homeflix-server:latest
   ```

## Environment Variables

### Required

| Variable | Description | Example |
|----------|-------------|---------|
| `MEDIA_DIR` | Path to media library root | `/media` or `/storage/media` |
| `TMDB_API_KEY` | TMDB API key for metadata | `YOUR_TMDB_API_KEY` |

### Optional

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | SQLite connection string | `sqlite:data.db?mode=rwc` |
| `PORT` | Server port | `3000` |
| `RUST_LOG` | Log level (error, warn, info, debug, trace) | `info` |
| `SCAN_INTERVAL_SECS` | Background scan interval in seconds | `3600` (1 hour) |

### Subtitle Generation (Optional)

| Variable | Description | Default |
|----------|-------------|---------|
| `WHISPER_MODEL_PATH` | Path to Whisper model file | `/app/models/ggml-small.bin` |
| `WHISPER_CLI_PATH` | Path to whisper-cli binary | `whisper-cli` |
| `OLLAMA_URL` | Ollama API URL for translation | `http://localhost:11434` |
| `OLLAMA_MODEL` | Ollama model name | `llama3.2` |

**Note:** Whisper models are automatically downloaded during Docker build. Available models: `tiny`, `base`, `small`, `medium`, `large`.

## Docker Compose Example

```yaml
services:
  server:
    build:
      context: .
      dockerfile: ./server/Dockerfile
    ports:
      - "3000:3000"
    volumes:
      - ./data:/data
      - /storage/media:/media
    environment:
      - DATABASE_URL=sqlite:/data/data.db?mode=rwc
      - MEDIA_DIR=/media
      - TMDB_API_KEY=your_tmdb_api_key
      - RUST_LOG=info
      # Optional: Whisper configuration
      - WHISPER_MODEL_PATH=/app/models/ggml-small.bin
      - WHISPER_CLI_PATH=whisper-cli
      # Optional: Ollama configuration
      - OLLAMA_URL=http://ollama:11434
      - OLLAMA_MODEL=llama3.2
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu, video, compute, utility]
```

## API Endpoints

- `GET /v2/library` - List all media
- `GET /v2/media/:id` - Get media details
- `GET /v2/media/:id/tracks` - Get audio/subtitle tracks
- `GET /v2/stream/web/:id` - Stream video (web player)
- `GET /v2/subtitles/:media_id/:index` - Get subtitle file
- `POST /v2/subtitles/:media_id/generate` - Generate subtitle
- `GET /v2/subtitles/capabilities` - Check Whisper/Ollama availability

## Features

- 🎬 Automatic media scanning and identification
- 🎥 Direct video streaming with FFmpeg transcoding
- 🎤 Multi-audio track support
- 📝 Automatic subtitle generation (Whisper + Ollama)
- 📊 Watch progress tracking
- 🎭 TMDB metadata integration
- 📺 TV series support with season/episode detection
- 🎞️ Collection presets for franchise timelines (Star Trek, Stargate, MCU, etc.)

## Collection Presets

The server supports collection presets defined in YAML format. Presets allow you to create curated franchise collections with proper timeline ordering, mixing movies and TV series.

### Preset Location

Presets are stored in the `presets/` directory within your data directory (same location as `data.db`). On first run, built-in presets are automatically copied from `server/presets/` to your data directory.

### YAML Format

Each preset is a YAML file with the following structure:

```yaml
name: "Collection Name"
description: "Collection description"
tmdb_collection_id: 12345  # Optional: TMDB collection ID for metadata
items:
  - tmdb_id: 314
    media_type: "tv"  # or "movie"
    title: "Series/Movie Title"
    timeline_order: 1
    timeline_year: 2151  # Optional: in-universe year
    timeline_notes: "Optional notes"  # Optional
    season_range:  # Optional: for TV series
      start: 1
      end: 2
```

### Creating Custom Presets

1. Create a new `.yaml` file in your `presets/` directory
2. Follow the format above
3. Use TMDB IDs for movies and TV shows (find them on [TMDB](https://www.themoviedb.org/))
4. Set `timeline_order` to define the viewing order
5. Restart the server or wait for the next scan cycle

### Example: Custom Preset

```yaml
name: "My Custom Franchise"
description: "A custom franchise timeline"
tmdb_collection_id: null
items:
  - tmdb_id: 12345
    media_type: "movie"
    title: "First Movie"
    timeline_order: 1
    timeline_year: 2000
    timeline_notes: null
    season_range: null
  - tmdb_id: 67890
    media_type: "tv"
    title: "TV Series"
    timeline_order: 2
    timeline_year: 2001
    timeline_notes: "Seasons 1-3"
    season_range:
      start: 1
      end: 3
```

### Built-in Presets

The server includes three built-in presets:
- **Star Trek Timeline** - Complete Star Trek franchise in chronological order
- **Stargate Timeline** - Stargate franchise timeline
- **MCU Timeline** - Marvel Cinematic Universe in chronological order

These are automatically copied to your presets directory on first run. You can modify them or create your own.

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Build release
cargo build --release
```

## Troubleshooting

**Whisper not available:**
- Ensure `whisper-cli` is in PATH or set `WHISPER_CLI_PATH`
- Check that model file exists at `WHISPER_MODEL_PATH`
- For Docker: GPU access required (`--gpus all` or docker-compose GPU config)

**Media not scanning:**
- Verify `MEDIA_DIR` is correctly set and accessible
- Check file permissions
- Review logs with `RUST_LOG=debug`

**Streaming issues:**
- Ensure FFmpeg is installed and in PATH
- Check video file codecs (H.264/H.265 recommended)
- Verify network connectivity for remote media
