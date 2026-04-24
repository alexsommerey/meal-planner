# Claude Working Notes — Meal Planner

## Planning

Always plan before coding. After we've agreed on direction, present a concrete plan — files to change, deps to add, order of steps — and wait for explicit approval before editing. A conceptual recommendation is not a plan.

## Commit style

**Conventional Commits.** `<type>: <subject>` — lowercase subject, no trailing period.

- `chore:` scaffolding, tooling, deps
- `feat:` user-visible capability
- `fix:` bug fix
- `refactor:` internal restructure with no behaviour change

Body explains *why*, not *what*, wrapped around 72 chars. Always include the `Co-Authored-By` trailer I'm configured to add.

## Branching workflow

`master` is protected — **never push directly**. Workflow for every change:

1. Branch from master: `git checkout -b <type>/<short-name>` (types match commit prefixes — `feat/`, `fix/`, `chore/`, `refactor/`).
2. If the session started you on a harness-generated branch (e.g. `claude/...`), **rename it first** (`git branch -m <type>/<short-name>`) — never push or open a PR from the auto-generated name.
3. Commit, push the branch.
4. Open a PR against master.
5. Merge via **squash** or **rebase** only (merge commits blocked).

Enforced on master: PR required, force-push blocked, deletion blocked, linear history required.

## Architecture

Cargo workspace with four crates, hexagonal layout:

- `crates/domain/` — pure types. No I/O, no async, no frameworks. Only serde/jiff/thiserror.
- `crates/application/` — use cases. Defines traits that `infrastructure` implements.
- `crates/infrastructure/` — adapters (DB, HTTP clients, file I/O). `sqlx` lives here when wired up.
- `crates/web/` — axum binary. Thin translation layer between HTTP and `application`.

Dependency direction: `web` → `application` + `infrastructure` → `domain`. Never reverse.

Add new deps at the **workspace root** in `[workspace.dependencies]`, then reference with `{ workspace = true }` in each crate's `Cargo.toml`. Keeps versions consistent across the workspace.

`Cargo.lock` **is committed** (workspace has a binary crate).
