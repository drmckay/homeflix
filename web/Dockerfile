# Build stage
FROM node:22-alpine AS builder

WORKDIR /app

# Install dependencies
COPY package.json package-lock.json ./
RUN npm ci

# Copy source files
COPY . .

# Build args for environment variables (optional - can be set at runtime)
# Note: VITE_API_URL is kept for backward compatibility, but PUBLIC_API_URL should be used at runtime
ARG VITE_API_URL=http://localhost:3000
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
# Runtime API URL - can be overridden at container startup
ENV PUBLIC_API_URL=http://localhost:3000

EXPOSE 3000

# Container metadata
LABEL org.opencontainers.image.source=https://github.com/drmckay/homeflix
LABEL org.opencontainers.image.description="HomeFlix Web Frontend"
LABEL org.opencontainers.image.licenses="MIT"

# Run the application directly with node (no npm/package.json needed)
CMD ["node", "build"]
