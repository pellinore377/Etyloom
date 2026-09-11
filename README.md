# Etyloom

**A workshop for languages with a past.**

Create a seeded language, inspect its dictionary and historical forms, explore its executable grammar, translate supported sentences, and practice against the same saved revision.

Rust · Axum · Leptos · Tailwind · SQLite · Pocket ID · Docker / Dockge.

## Release scope

This is an **early functional release**, not the completed research roadmap. The engine is implemented in Rust; generation does not call an AI service. Every page uses persisted application data rather than a mock language.

Implemented:

- Addressed, deterministic randomness; versioned recipes and content checksums.
- Six constituent orders, particle/suffix/mixed morphology, adjective placement, optional object case, pronouns and number.
- Typed sentence meanings, tense, negation and modal scope, polar questions and coordination.
- Generation of ancestral roots and complete inflected paradigms, five sound-change mechanisms, inherited alternations, frequency-biased analogy, and dated modern compounds.
- 128–5,000 lexical entries. Large lexicons include **transparent generated compounds**, not thousands of curated independent root meanings.
- Projects, persistent generation jobs, cancellation and recovery, drafts, immutable published revisions, exports and validated package import.
- Grammar-backed bidirectional **controlled** translation. Unsupported input fails explicitly; alternative analyses remain visible.
- Revision-bound exercises and saved answer history; structurally split, bidirectional JSONL corpus exports.
- Fieldnotes interface with brass accents, responsive layouts, and persistent light/dark themes.
- Pocket ID sign-in, project authorization, database-backed sessions, CSRF checks, and non-root Docker deployment.

Not implemented: unrestricted neural translation in the web app, language-family authoring, arbitrary custom history rules, rich contact/borrowing simulation, relative clauses, different-subject complements, aspect, possession, comparison, tone, pronunciation audio, or a complete adaptive curriculum. Community notes are reference material, not automatically interpreted generation inputs. See [coverage](docs/coverage.md).

## Dockge deployment

1. Register an OIDC client in your existing Pocket ID. Callback: `https://your-etyloom-domain/auth/callback`. Enable PKCE and authorize the users who should access Etyloom.
2. In Dockge, create an `etyloom` stack using `compose.yaml` and populate its environment from `.env.example`.
3. Put the application behind an HTTPS reverse proxy. Point the proxy to `127.0.0.1:8080` when it runs on the host. The default port binding is loopback-only.
4. Deploy. The `/data` named volume preserves the database, sessions, jobs and language packages.

The application expects **your Pocket ID to exist already**. It does not run another identity provider or require Docker socket access. Production refuses insecure HTTP configuration or development authentication.

The default Compose file uses the prebuilt image published by successful release CI. An image tag is usable only after its publish workflow finishes. Pin a verified image digest for controlled upgrades.

To build locally instead:

```sh
cp .env.example .env
# Set the real public origin and Pocket ID client credentials.
docker compose -f compose.yaml -f compose.build.yaml up -d --build
```

For a reverse proxy in another container, attach both services to a shared Docker network and proxy to `etyloom:3000`, or configure an explicitly appropriate host binding. Do not expose the plain HTTP port publicly without a trusted HTTPS proxy.

## Local development

Install the pinned Rust toolchain, Node 22, and cargo-leptos 0.3.7.

```sh
cargo install cargo-leptos --version 0.3.7 --locked
npm ci
npm run css
ETYLOOM_DEV_AUTH=true \
BASE_URL=http://127.0.0.1:3000 \
LEPTOS_SITE_ADDR=127.0.0.1:3000 \
cargo leptos watch
```

Development sign-in exists only in **debug builds on loopback**. There is no release-mode authentication bypass. The web interface starts at `http://127.0.0.1:3000`.

## Headless engine

```sh
cargo run -p etyloom-cli -- recipe > recipe.json
cargo run -p etyloom-cli -- generate recipe.json > language.json
cargo run -p etyloom-cli -- validate language.json
cargo run -p etyloom-cli -- translate language.json 'I see the river'
cargo run -p etyloom-cli -- corpus language.json 10000 > corpus.jsonl
cargo run --release -p etyloom-cli -- bench 4096 20
```

A recipe includes the seed, settings and engine/content versions. A full package additionally preserves the generated grammar, all recorded paradigms, examples and revision checksum. Keep full packages as durable exports; future versions must not pretend an old seed is reproducible with changed algorithms.

## Verification

```sh
cargo fmt --all -- --check
cargo test -p etyloom-engine --locked
cargo test -p etyloom-web --features ssr --locked
cargo check -p etyloom-web --lib --features hydrate --target wasm32-unknown-unknown --locked
cargo clippy --workspace --all-targets --features etyloom-web/ssr --locked -- -D warnings
npm run test:browser
```

CI retains actual compiler, test, benchmark and browser reports. Passing a source review is not a substitute for these checks. Generation targets remain workload-specific; the benchmark is an early-release engine workload, **not proof that the full future grammar coverage meets its 30-second goal**.

## Backups

Create a transactionally consistent SQLite backup inside the data volume:

```sh
docker compose exec etyloom etyloom-web backup /data/backup-2026-09-11.db
docker compose cp etyloom:/data/backup-2026-09-11.db ./backup-2026-09-11.db
```

The destination must not already exist. Protect backups: they contain private languages and login sessions. A backup is not useful until a restore is tested. See [operations](docs/operations.md).

## Architecture and contribution rules

- `crates/core`: portable typed model and API contracts.
- `crates/engine`: generation, forms, history, parsing, realization, corpus and validation.
- `crates/cli`: headless engine and benchmark entry points.
- `crates/web`: Leptos browser UI plus feature-gated Axum, storage, jobs and OIDC.
- `style`: semantic light/dark tokens and Tailwind styling.

Use terse, idiomatic Rust, explicit error handling, and minimal comments that explain only non-obvious reasons or invariants. Run rustfmt; do not conceal failures with default values. Put architectural explanation in documentation and behavioral proof in tests.

Original editorial concept content and code are MIT licensed. No external phonological or lexical datasets, proprietary fonts, third-party mockup assets, or generated model weights are bundled.
