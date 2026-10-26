# AGENTS.md

Guidance for AI coding agents working in this repository.

## Stack

Server-rendered web app. No frontend framework, no Node, no frontend build step.

| Layer | Choice |
|---|---|
| Runtime | `tokio` |
| HTTP | `axum` (routes, extractors, responses) |
| (De)serialization | `serde` (forms, query strings) |
| Database | `sqlx` with PostgreSQL, plain SQL, migrations in `migrations/` |
| HTML | `askama` templates in `templates/`, checked at compile time |
| Static files and request logs | `tower-http` (`ServeDir`, `TraceLayer`) |
| Logging | `tracing` + `tracing-subscriber` |
| CSS | Modern vanilla CSS: tokens, `@layer`, nesting, `@scope` |
| JS | Vanilla JS, only where needed, in `static/app.js` |

Do not add dependencies (crates, JS libraries, CSS frameworks) without asking.

## Commands

```sh
cargo run                              # run the app on http://127.0.0.1:3000
cargo check                            # fast compile check
cargo fmt                              # format
cargo clippy --all-targets -- -D warnings
cargo test
```

## Conventions

### Rust

- Handlers return `Result<_, AppError>`. Unexpected errors become a logged 500.
- Validation errors re-render the page with a message and a 400 (see
  `error_page`). Always validate on the server; browser checks are optional UX.
- Forms use Post/Redirect/Get: a successful `POST` answers with a redirect.
- **GET routes must not have side effects.** `templates/base.html` prerenders
  same-origin links (Speculation Rules), so the browser may request a GET
  route before the user clicks. Anything that writes data uses `POST`.
- Put pure logic (validation, parsing) in plain functions and unit test them in
  the `tests` module.



Before finishing any task, run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`
and `cargo test`. All three must pass (CI runs the same commands).
