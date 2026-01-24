# Build stage
FROM node:22-alpine AS builder

WORKDIR /app

# Install dependencies
COPY package.json package-lock.json ./
RUN npm ci

# Copy source files
COPY . .

# Build args for environment variables baked into the build
ARG VITE_API_URL
ENV VITE_API_URL=${VITE_API_URL}

# Build the application
RUN npm run build

# Runtime stage - minimal image with only the built application
FROM node:22-alpine AS runtime

WORKDIR /app

# Create non-root user for security
RUN addgroup --system --gid 1001 nodejs && \
    adduser --system --uid 1001 sveltekit

# Copy only the built application (adapter-node bundles all dependencies)
COPY --from=builder --chown=sveltekit:nodejs /app/build build/

# Switch to non-root user
USER sveltekit

# Environment variables
ENV HOST=0.0.0.0
ENV PORT=3000
ENV NODE_ENV=production

EXPOSE 3000

# Container metadata
LABEL org.opencontainers.image.source=https://github.com/drmckay-dev/homeflix
LABEL org.opencontainers.image.description="HomeFlix Web Frontend"
LABEL org.opencontainers.image.licenses="MIT"

# Run the application directly with node (no npm/package.json needed)
CMD ["node", "build"]
