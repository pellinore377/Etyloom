use super::{AppState, auth::User, error::{AppError, Result}};
use etyloom_core::{Job, LanguageSummary, Package};
use std::sync::Arc;

#[derive(Debug, sqlx::FromRow)]
pub struct LanguageRow {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub revision: String,
    pub status: String,
    pub seed: String,
    pub entry_count: i64,
    pub stage_count: i64,
    pub word_order: String,
}

impl LanguageRow {
    pub fn summary(self) -> LanguageSummary {
        LanguageSummary { id: self.id, project_id: self.project_id, name: self.name, seed: self.seed, revision: self.revision, status: self.status, entries: self.entry_count.max(0) as usize, stages: self.stage_count.max(0) as usize, order: self.word_order.to_uppercase() }
    }
}

pub const LANGUAGE_QUERY: &str = "SELECT l.id,l.project_id,l.name,coalesce(l.draft_revision,l.published_revision) AS revision,CASE WHEN l.draft_revision IS NULL THEN 'published' ELSE 'draft' END AS status,r.seed,r.entry_count,r.stage_count,r.word_order FROM languages l JOIN projects p ON p.id=l.project_id JOIN revisions r ON r.hash=coalesce(l.draft_revision,l.published_revision)";

pub async fn owned_language(state: &AppState, user: &User, id: &str) -> Result<LanguageRow> {
    sqlx::query_as(&format!("{LANGUAGE_QUERY} WHERE l.id=? AND p.owner_id=?"))
        .bind(id).bind(user.id).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)
}

pub async fn owned_project(state: &AppState, user: &User, id: &str) -> Result<()> {
    let found: Option<(String,)> = sqlx::query_as("SELECT id FROM projects WHERE id=? AND owner_id=?").bind(id).bind(user.id).fetch_optional(&state.db).await?;
    found.map(|_| ()).ok_or(AppError::NotFound)
}

pub async fn package(state: &AppState, revision: &str) -> Result<Arc<Package>> {
    if let Some(package) = state.cache.read().await.get(revision).cloned() { return Ok(package); }
    let (json,): (String,) = sqlx::query_as("SELECT package FROM revisions WHERE hash=?").bind(revision).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;
    let package: Package = tokio::task::spawn_blocking(move || serde_json::from_str(&json)).await.map_err(|e| AppError::Internal(e.into()))??;
    let package = Arc::new(package);
    let mut cache = state.cache.write().await;
    if cache.len() >= 4 { cache.pop_first(); }
    cache.insert(revision.into(), package.clone());
    Ok(package)
}

pub async fn owned_package(state: &AppState, user: &User, id: &str) -> Result<Arc<Package>> {
    let language = owned_language(state, user, id).await?;
    package(state, &language.revision).await
}

pub async fn owned_revision(state: &AppState, user: &User, id: &str, revision: &str) -> Result<Arc<Package>> {
    owned_language(state, user, id).await?;
    let exists: Option<(String,)> = sqlx::query_as("SELECT revision FROM language_revisions WHERE language_id=? AND revision=?").bind(id).bind(revision).fetch_optional(&state.db).await?;
    if exists.is_none() { return Err(AppError::NotFound); }
    package(state, revision).await
}

#[derive(sqlx::FromRow)]
pub struct JobRow { pub id: String, pub state: String, pub phase: String, pub language_id: Option<String>, pub error: Option<String> }
impl From<JobRow> for Job {
    fn from(row: JobRow) -> Job { Job { id: row.id, state: row.state, phase: row.phase, language_id: row.language_id, error: row.error } }
}

pub async fn insert_revision(transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>, package: &Package, json: &str) -> Result<()> {
    sqlx::query("INSERT INTO revisions(hash,package,seed,entry_count,stage_count,word_order) VALUES(?,?,?,?,?,?) ON CONFLICT(hash) DO NOTHING")
        .bind(&package.revision).bind(json).bind(&package.recipe.seed).bind(package.lexicon.len() as i64).bind(package.stages.len() as i64).bind(package.grammar.order.to_string()).execute(&mut **transaction).await?;
    Ok(())
}
