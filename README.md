# ForgeTrack

ForgeTrack is a self-hosted project management platform inspired by Redmine, Linear, and GitHub Issues. It is built as a modular Rust monolith designed to evolve into a scalable, distributed system.

## Tech Stack

### Backend

- Rust
- Axum
- Tokio
- SQLx
- PostgreSQL

### Frontend

- Next.js (App Router; not a standalone React application)
- TypeScript
- TanStack Query
- Zustand
- Tailwind CSS

During Phase 1, run the frontend natively with `next dev`. Docker development and
production images are added after the core user flow is working.

### Infrastructure

- Docker Compose

## Run Phase 1

Native development comes first:

1. Start PostgreSQL and create a local `.env` from `.env.example` with a valid `DATABASE_URL` and `JWT_SECRET` for the API.
2. Run migrations with `cargo run --package api -- migrate` from `backend/`.
3. Start the API with `cargo run --package api` from `backend/` and the UI with `npm run dev` from `frontend/`.

After the native flow is working, copy `.env.example` to `.env` and run `docker compose up --build`. To use hot reload in containers, copy `docker-compose.override.yml.example` to the ignored `docker-compose.override.yml` file.
