# HomeFlix Web Frontend

A Netflix-like web interface for your personal media library, built with SvelteKit and Tailwind CSS.

## Features

- Browse movies and TV shows with a familiar Netflix-style interface
- Hero banner with featured content
- Movie and series detail modals with metadata
- Video player with HLS streaming support
- Multi-language subtitle support with auto-generation
- Search functionality
- Collections support
- Responsive design for desktop and mobile
- Internationalization (English, Hungarian)

## Tech Stack

- **Framework:** SvelteKit 2 with Svelte 5
- **Styling:** Tailwind CSS 4
- **Video:** Vidstack player with HLS.js
- **i18n:** Paraglide
- **Testing:** Vitest + Playwright

## Development

### Prerequisites

- Node.js 22+
- npm

### Setup

```bash
# Install dependencies
npm install

# Start development server
npm run dev
```

The app will be available at `http://localhost:5173`

### Environment Variables

Create a `.env` file or set these variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `VITE_API_URL` | Backend API URL | `http://localhost:3000` |

### Scripts

```bash
npm run dev        # Start dev server
npm run build      # Production build
npm run preview    # Preview production build
npm run check      # TypeScript + Svelte check
npm run lint       # ESLint + Prettier check
npm run format     # Auto-format code
npm run test:unit  # Run unit tests
npm run test:e2e   # Run e2e tests
```

## Docker

### Build

```bash
# Build-time API URL is optional (can be overridden at runtime)
docker build -t homeflix-web --build-arg VITE_API_URL=http://api:3000 .
```

### Run

```bash
# Runtime API URL can be set via environment variable
docker run -d -p 3000:3000 -e PUBLIC_API_URL=http://api:3000 homeflix-web
```

### Environment Variables (Runtime)

| Variable | Description | Default |
|----------|-------------|---------|
| `HOST` | Server bind address | `0.0.0.0` |
| `PORT` | Server port | `3000` |
| `PUBLIC_API_URL` | Backend API URL (runtime configurable) | `http://localhost:3000` |
| `ORIGIN` | Allowed origin for CORS | - |

**Note:** The `PUBLIC_API_URL` environment variable can be set at runtime (e.g., in Docker) and will override any build-time `VITE_API_URL` value. This allows the same Docker image to be used with different backend URLs without rebuilding.

## Project Structure

```
src/
├── routes/           # SvelteKit routes
│   ├── +page.svelte  # Home page
│   ├── movies/       # Movies listing
│   ├── shows/        # TV shows listing
│   └── collections/  # Collections pages
├── lib/
│   ├── components/   # Svelte components
│   ├── api.ts        # Backend API client
│   └── types.ts      # TypeScript types
└── messages/         # i18n translations
```

## License

MIT
