# Deployment and operations

## Security boundary

Pocket ID is the identity provider. Etyloom uses authorization-code flow with S256 PKCE, nonce/state verification, validated ID tokens and a browser-bound pending-login cookie. Identity is keyed by issuer and subject, not display name. The application then creates an independent opaque session, stores only the session-token hash, rotates login sessions, and drops provider tokens.

Sessions are HTTP-only, SameSite=Lax, Secure in production, and persisted in SQLite. They expire after fourteen days. Writes require a session-specific CSRF header; an Origin header, when present, must match BASE_URL. Every project, language, revision, export and job read/write checks ownership server-side. The application has no public project-sharing feature in this release.

Production requires HTTPS BASE_URL and issuer. Configure trusted TLS termination yourself. No OIDC secret belongs in the Leptos WASM bundle. Configure Pocket ID client access restrictions; users authorized by that client may create their own isolated workspace. Gate the deployment at your reverse proxy as well when an additional allowlist is needed.

Opaque session tokens come from operating-system randomness. Language seeds never serve as authentication secrets. Logs omit HTTP headers and serialized bodies. Application errors expose a correlation reference, not SQL or identity-token contents.

## Storage

One application instance owns one SQLite database. Do not scale this deployment to multiple containers sharing a database. WAL, foreign keys, bounded connection pools and transactional publication are enabled. Language packages are immutable revision records inside the database; caches can be discarded.

The default Docker image runs as UID/GID 10001. The named data volume inherits writable ownership. For a host bind mount instead, explicitly grant that UID access. The root filesystem is read-only, writable scratch is confined to /tmp, and the container has no Linux capabilities or Docker socket.

The app consumes existing projects and jobs locally. It does not send language content to external generation services. Pocket ID network requests occur during authentication.

## Jobs and recovery

Generation is off the request executor, in a bounded blocking worker. By default one language job runs at a time, with at most two active jobs per user and sixteen globally. The UI receives actual phase changes, never a fabricated progress percentage. Cancellation is checked between bounded phases and before publication.

After restart, running jobs return to the queue and replay their saved recipes. This release recomputes rather than resuming an intermediate binary checkpoint. Cancellation requests become canceled. Published language revisions remain unchanged.

## Backup and restore

Use `etyloom-web backup /data/new-backup.db` to perform SQLite VACUUM INTO. Copy the resulting database out of the volume. Never copy only a live WAL-mode main database file and assume it is consistent.

To restore: stop Etyloom, preserve a copy of the existing data directory, replace the database with the backup while removing stale -wal/-shm companions, restore UID/GID 10001 ownership, then restart the same application version. Verify login, project listing, an exported revision checksum and a sample translation before upgrading.

Backups contain private work, pending authentication records and session metadata. Encrypt or access-control them. After disaster recovery, invalidate sessions by deleting sessions and logins through an administrative SQLite maintenance connection while the app is stopped if their integrity is uncertain.

## Updates

Pin a tested image digest. Back up before upgrading. Schema migrations fail explicitly at startup; they never replace unreadable authored data with defaults. Restore the previous image and backup together if a migration is not backward compatible. Keep exported language packages separately from database backups.

`/health/live` is a process probe; `/health/ready` checks the local database. Readiness deliberately does not depend on Pocket ID availability, so an identity-provider outage does not take down already authenticated work. Login itself reports a recoverable provider failure.

## Development access

ETYLOOM_DEV_AUTH=true is accepted only by debug builds, with a loopback listener and loopback BASE_URL. It is rejected by the release binary. Do not alter that guard to make a production deployment easier.

## Performance

Run `etyloom bench 4096 20` in a release build. Record CPU model, memory, architecture, concurrency and full content version with results. This early benchmark covers the mechanisms in docs/coverage.md. It is not proof of the full future language-generation complexity or multi-user latency target.
