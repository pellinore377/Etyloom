CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE(issuer, subject)
);
CREATE TABLE sessions (
    token_hash TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    csrf TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);
CREATE INDEX sessions_expiry ON sessions(expires_at);
CREATE TABLE logins (
    token_hash TEXT PRIMARY KEY,
    state TEXT NOT NULL,
    nonce TEXT NOT NULL,
    pkce TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    owner_id INTEGER NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX projects_owner ON projects(owner_id);
CREATE TABLE revisions (
    hash TEXT PRIMARY KEY,
    package TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE languages (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    name TEXT NOT NULL,
    published_revision TEXT REFERENCES revisions(hash),
    draft_revision TEXT REFERENCES revisions(hash),
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    CHECK(published_revision IS NOT NULL OR draft_revision IS NOT NULL)
);
CREATE INDEX languages_project ON languages(project_id);
CREATE TABLE language_revisions (
    language_id TEXT NOT NULL REFERENCES languages(id),
    revision TEXT NOT NULL REFERENCES revisions(hash),
    parent_revision TEXT REFERENCES revisions(hash),
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY(language_id, revision)
);
CREATE TABLE jobs (
    id TEXT PRIMARY KEY,
    owner_id INTEGER NOT NULL REFERENCES users(id),
    project_id TEXT NOT NULL REFERENCES projects(id),
    language_id TEXT REFERENCES languages(id),
    base_revision TEXT,
    recipe TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','running','cancel_requested','canceled','completed','failed')),
    phase TEXT NOT NULL DEFAULT 'Waiting for a worker',
    error TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX jobs_queue ON jobs(state, created_at);
CREATE INDEX jobs_owner ON jobs(owner_id, created_at);
CREATE UNIQUE INDEX jobs_language_active ON jobs(language_id) WHERE state IN ('queued','running','cancel_requested');
CREATE TABLE reviews (
    user_id INTEGER NOT NULL REFERENCES users(id),
    language_id TEXT NOT NULL REFERENCES languages(id),
    revision TEXT NOT NULL REFERENCES revisions(hash),
    exercise INTEGER NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    correct INTEGER NOT NULL DEFAULT 0,
    last_review INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY(user_id, language_id, revision, exercise)
);
