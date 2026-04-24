# meal-planner

A meal-planning web app. Early-stage scaffolding — cargo workspace, hexagonal
layout, OpenTelemetry wired up, not much domain yet.

## Prerequisites

- [rustup](https://rustup.rs/) (pinned via `rust-toolchain.toml`)
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) for the
  observability stack

## Run

Web server (listens on `:3000`, or `$PORT` if set):

```
cargo run -p web
```

Observability stack (Jaeger for traces, Prometheus for metrics, shared
OTel collector):

```
docker compose --profile traces up    # collector + jaeger
docker compose --profile metrics up   # collector + prometheus
```

UIs:
- Web — http://localhost:3000
- Health — http://localhost:3000/healthz
- Jaeger — http://localhost:16686
- Prometheus — http://localhost:9090

## Contributing

Once per clone, point git at the tracked hooks so `cargo fmt`/`clippy` gate
commits and `cargo test` gates pushes:

```
git config core.hooksPath .githooks
```

Branching/commit conventions, architecture notes, and the OTel wiring are
all in [CLAUDE.md](CLAUDE.md).
