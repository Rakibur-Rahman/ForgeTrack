# ForgeTrack Architecture Decision Log

This document records important architectural and implementation decisions throughout the project.

---

## Phase 1 — Foundation

### D001 — Modular Monolith Architecture

**Decision**

ForgeTrack will be implemented as a modular monolith using Rust workspaces and independent feature crates.

**Reason**

- Clear separation of business domains.
- Easier development and testing than microservices.
- Allows gradual extraction into services later.

---

### D002 — Native-First Development, Containerize Later

**Decision

For Phase 1 steps 1–6, ForgeTrack runs natively: PostgreSQL runs locally (or in a single Docker container), the backend runs with `cargo run`, and the frontend runs with `next dev`.

Dockerfiles and Docker Compose are introduced only after the core application flow works, in Phase 1 steps 7–8.

**Reason

This keeps the early development loop fast while the database schema and API design are still changing. Containerization is added once the native stack is working, giving the project a repeatable setup without slowing initial development.

---

### D003 — PostgreSQL as the Primary Database

**Decision**

Use PostgreSQL with SQLx for compile-time checked SQL queries.

**Reason**

- Strong relational model.
- Excellent full-text search support.
- Reliable migrations.
- Production-ready performance.

---

## Future Decisions

New decisions will be added as the project evolves through each phase.