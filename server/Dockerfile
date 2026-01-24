# =============================================================================
# Multi-stage Dockerfile with NVIDIA GPU support
# - Rust application
# - Whisper.cpp with CUDA
# - FFmpeg with NVIDIA (NVENC/NVDEC) + LibTorch (demucs)
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Rust Builder
# -----------------------------------------------------------------------------
FROM rust:1.92.0-slim-bookworm AS rust-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy the local dependency first
COPY media-identifier /media-identifier

# Copy manifests
COPY server/Cargo.toml server/Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    touch src/api.rs src/db.rs src/models.rs src/scanner.rs src/streaming.rs

# Build dependencies
RUN cargo build --release
RUN rm src/*.rs

# Copy source code
COPY server/src ./src

# Build application
RUN touch src/main.rs
RUN cargo build --release

# -----------------------------------------------------------------------------
# Stage 2: Whisper.cpp Builder (with CUDA support)
# -----------------------------------------------------------------------------
FROM nvidia/cuda:12.2.0-devel-ubuntu22.04 AS whisper-builder

WORKDIR /build

ENV DEBIAN_FRONTEND=noninteractive

# Install whisper.cpp build dependencies
RUN apt-get update && apt-get install -y \
    git \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Clone and build whisper.cpp with CUDA support
# Only build whisper-cli (skip examples that require CUDA driver at link time)
# Use GGML_CUDA_NO_VMM to avoid linking with libcuda.so.1 (driver library) at build time
# The driver will be available at runtime when container has GPU access
RUN git clone https://github.com/ggerganov/whisper.cpp.git . && \
    cmake -B build \
        -DCMAKE_BUILD_TYPE=Release \
        -DGGML_CUDA=ON \
        -DGGML_CUDA_NO_VMM=ON \
        -DCMAKE_CUDA_ARCHITECTURES="75;80;86;89;90" && \
    cmake --build build --config Release --target whisper-cli -j$(nproc)

# -----------------------------------------------------------------------------
# Stage 3: FFmpeg Builder (NVIDIA + LibTorch)
# -----------------------------------------------------------------------------
FROM nvidia/cuda:12.2.0-devel-ubuntu22.04 AS ffmpeg-builder

ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /build

# Build tools and FFmpeg dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    git \
    nasm \
    yasm \
    pkg-config \
    wget \
    unzip \
    # FFmpeg codec dependencies
    libx264-dev \
    libx265-dev \
    libvpx-dev \
    libfdk-aac-dev \
    libmp3lame-dev \
    libopus-dev \
    libvorbis-dev \
    libass-dev \
    libfreetype6-dev \
    libssl-dev \
    libsoxr-dev \
    libopenblas-dev \
    # Chromaprint for audio fingerprinting
    libchromaprint-dev \
    && rm -rf /var/lib/apt/lists/*

# Download LibTorch (CUDA version)
ARG LIBTORCH_VERSION=2.1.0
ARG CUDA_VERSION_SHORT=cu121

RUN wget -q "https://download.pytorch.org/libtorch/${CUDA_VERSION_SHORT}/libtorch-cxx11-abi-shared-with-deps-${LIBTORCH_VERSION}%2B${CUDA_VERSION_SHORT}.zip" \
    -O libtorch.zip && \
    unzip -q libtorch.zip -d /opt && \
    rm libtorch.zip

ENV LIBTORCH_PATH=/opt/libtorch
ENV CMAKE_PREFIX_PATH=/opt/libtorch
ENV LD_LIBRARY_PATH=/opt/libtorch/lib:$LD_LIBRARY_PATH

# Install nv-codec-headers (NVIDIA Video Codec SDK)
ARG NV_CODEC_VERSION=12.1.14.0

RUN git clone --depth 1 --branch n${NV_CODEC_VERSION} \
    https://github.com/FFmpeg/nv-codec-headers.git && \
    cd nv-codec-headers && \
    make install && \
    cd .. && rm -rf nv-codec-headers

# Download and extract FFmpeg source
ARG FFMPEG_VERSION=8.0.1

RUN wget -q "https://ffmpeg.org/releases/ffmpeg-${FFMPEG_VERSION}.tar.xz" && \
    tar xf ffmpeg-${FFMPEG_VERSION}.tar.xz && \
    rm ffmpeg-${FFMPEG_VERSION}.tar.xz

WORKDIR /build/ffmpeg-${FFMPEG_VERSION}

# Configure FFmpeg with NVIDIA + LibTorch support
RUN ./configure \
    --prefix=/usr/local \
    --enable-gpl \
    --enable-nonfree \
    --enable-version3 \
    # NVIDIA hardware acceleration
    --enable-cuda-nvcc \
    --enable-cuvid \
    --enable-nvenc \
    --enable-nvdec \
    --enable-libnpp \
    --extra-cflags="-I/usr/local/cuda/include" \
    --extra-ldflags="-L/usr/local/cuda/lib64" \
    # LibTorch for demucs filter
    --enable-libtorch \
    --extra-cflags="-I/opt/libtorch/include -I/opt/libtorch/include/torch/csrc/api/include" \
    --extra-ldflags="-L/opt/libtorch/lib -Wl,-rpath,/opt/libtorch/lib" \
    # Codecs
    --enable-libx264 \
    --enable-libx265 \
    --enable-libvpx \
    --enable-libfdk-aac \
    --enable-libmp3lame \
    --enable-libopus \
    --enable-libvorbis \
    --enable-libass \
    --enable-libfreetype \
    --enable-libsoxr \
    --enable-chromaprint \
    --enable-openssl \
    # Build options
    --enable-shared \
    --disable-static \
    --disable-debug \
    --disable-doc

# Build FFmpeg
RUN make -j$(nproc) && make install

# -----------------------------------------------------------------------------
# Stage 4: Runtime
# -----------------------------------------------------------------------------
FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04

WORKDIR /app

ENV DEBIAN_FRONTEND=noninteractive
ENV NVIDIA_VISIBLE_DEVICES=all
ENV NVIDIA_DRIVER_CAPABILITIES=compute,utility,video

# Install runtime dependencies (CUDA libs removed - will copy from builder)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    libsqlite3-0 \
    # FFmpeg codec runtime libraries
    libx264-163 \
    libx265-199 \
    libvpx7 \
    libfdk-aac2 \
    libmp3lame0 \
    libopus0 \
    libvorbis0a \
    libvorbisenc2 \
    libass9 \
    libfreetype6 \
    libsoxr0 \
    libchromaprint1 \
    # LibTorch dependencies
    libopenblas0 \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# Copy CUDA libraries from whisper builder (devel image has them)
COPY --from=whisper-builder /usr/local/cuda/lib64/libcublas*.so* /usr/local/lib/
COPY --from=whisper-builder /usr/local/cuda/lib64/libcudart*.so* /usr/local/lib/

# Copy FFmpeg binaries and libraries
COPY --from=ffmpeg-builder /usr/local/bin/ffmpeg /usr/local/bin/
COPY --from=ffmpeg-builder /usr/local/bin/ffprobe /usr/local/bin/
COPY --from=ffmpeg-builder /usr/local/lib/lib*.so* /usr/local/lib/

# Copy LibTorch runtime libraries
COPY --from=ffmpeg-builder /opt/libtorch/lib/*.so* /usr/local/lib/

# Copy whisper-cli binary with CUDA support
COPY --from=whisper-builder /build/build/bin/whisper-cli /usr/local/bin/whisper-cli

# Copy whisper and GGML CUDA libraries
COPY --from=whisper-builder /build/build/src/libwhisper*.so* /usr/local/lib/
COPY --from=whisper-builder /build/build/ggml/src/libggml*.so* /usr/local/lib/
COPY --from=whisper-builder /build/build/ggml/src/ggml-cuda/libggml*.so* /usr/local/lib/

# Update library cache
RUN ldconfig

# Create directories
RUN mkdir -p /data /data/.cache /data/presets /app/models /app/presets-default

# Download Whisper model (large-v3 for best quality)
ARG WHISPER_MODEL=large-v3
RUN curl -L -o /app/models/ggml-${WHISPER_MODEL}.bin \
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-${WHISPER_MODEL}.bin"

# Copy Rust binary from builder
COPY --from=rust-builder /app/target/release/homeflixd ./server

# Copy presets to default location (will be copied to /data/presets on startup if not present)
COPY server/presets /app/presets-default

# Create entrypoint script to initialize presets
RUN echo '#!/bin/sh\n\
# Copy default presets if /data/presets is empty\n\
if [ -z "$(ls -A /data/presets 2>/dev/null)" ]; then\n\
    echo "Initializing presets from defaults..."\n\
    cp -r /app/presets-default/* /data/presets/\n\
fi\n\
exec "$@"' > /app/entrypoint.sh && chmod +x /app/entrypoint.sh

# Environment variables
ENV DATABASE_URL="sqlite:/data/data.db?mode=rwc"
ENV CACHE_DIR="/data/.cache"
ENV PRESETS_PATH="/data/presets"
ENV PORT=3000
ENV RUST_LOG=info

# Whisper configuration
ENV WHISPER_MODEL_PATH="/app/models/ggml-large-v3.bin"
ENV WHISPER_CLI_PATH="whisper-cli"

# Ollama configuration
ENV OLLAMA_URL="http://ollama:11434"
ENV OLLAMA_MODEL="llama3.2"

# Verify installations
RUN echo "=== FFmpeg ===" && ffmpeg -version && \
    echo "=== NVIDIA HWAccel ===" && ffmpeg -hwaccels 2>&1 | head -10 && \
    echo "=== Whisper ===" && whisper-cli --help 2>&1 | head -5 || true

EXPOSE 3000

# Healthcheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["./server"]