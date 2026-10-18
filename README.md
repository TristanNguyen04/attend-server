# attend-server

Backend of the SUTD NFC attendance system. A modular monolith on [Loco](https://loco.rs)
(Rust, SeaORM, Axum, Postgres).

Students use a native app (one device per account); TAs and professors use a web app. Both
live in separate repositories and talk to this API.

## Quick start

You need Rust (the exact version is pinned in `rust-toolchain.toml`; `rustup` installs it) and
Docker.

```
docker compose up -d      # Postgres on localhost:5440 (development and test databases)
cargo test                # unit tests, architecture rules, and the Cucumber scenarios
cargo loco start          # run the app on http://localhost:5150
curl localhost:5150/_ping # {"ok":true}
```

If something else already uses port 5440, set `POSTGRES_PORT` for Docker and `DATABASE_URL`
for the app to match.

## Where to look

| I want to... | Read |
|---|---|
| Understand how the code is organised | `docs/architecture.md` |
| See the data model | `docs/er-diagram.md` |
| Build a feature the way we do it here | `docs/development-process.md`, `constitution.md` |
| Write a spec | `specs/README.md` and `specs/_templates/` |
| Write a scenario | `tests/features/README.md` |
| Work with an AI assistant | `AGENTS.md` |

## Layout

```
src/modules/<module>/   calendar, identity, courses, capture, records
src/shared_lib/         event envelope, clock, error envelope, policies contract
migration/              migrations, one folder per module
specs/                  requirements and tasks, per module and feature
tests/                  features (Gherkin), steps, architecture rules
docs/                   architecture, data model, process
```

## Checks before a pull request

```
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same three commands. Changes reach `main` through pull requests.

## Status

This is the scaffold: the structure, tooling and test harness. Tables, features, the event
outbox, authentication and deployment files are added feature by feature.
