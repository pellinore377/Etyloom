use super::{
    AppState,
    auth::{User, random_token},
    error::{AppError, Result},
    store::{self, JobRow, LanguageRow},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use etyloom_core::*;
use serde::{Deserialize, Serialize};

pub async fn projects(State(state): State<AppState>, user: User) -> Result<Json<Vec<Project>>> {
    let rows: Vec<(String,String,i64)> = sqlx::query_as("SELECT p.id,p.name,count(l.id) FROM projects p LEFT JOIN languages l ON l.project_id=p.id WHERE p.owner_id=? GROUP BY p.id ORDER BY p.created_at,p.id").bind(user.id).fetch_all(&state.db).await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, name, language_count)| Project {
                id,
                name,
                language_count,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectInput {
    name: String,
}

pub async fn create_project(
    State(state): State<AppState>,
    user: User,
    headers: HeaderMap,
    Json(input): Json<ProjectInput>,
) -> Result<Json<Project>> {
    user.csrf(&headers, &state)?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(AppError::Invalid(
            "Project name must contain 1–80 characters".into(),
        ));
    }
    let id = random_token()?;
    sqlx::query("INSERT INTO projects(id,owner_id,name) VALUES(?,?,?)")
        .bind(&id)
        .bind(user.id)
        .bind(name)
        .execute(&state.db)
        .await?;
    Ok(Json(Project {
        id,
        name: name.into(),
        language_count: 0,
    }))
}

#[derive(Deserialize)]
pub struct ProjectFilter {
    project: Option<String>,
}

pub async fn languages(
    State(state): State<AppState>,
    user: User,
    Query(filter): Query<ProjectFilter>,
) -> Result<Json<Vec<LanguageSummary>>> {
    let rows: Vec<LanguageRow> = sqlx::query_as(&format!("{} WHERE p.owner_id=? AND (? IS NULL OR l.project_id=?) ORDER BY l.created_at DESC,l.id LIMIT 200", store::LANGUAGE_QUERY))
        .bind(user.id).bind(&filter.project).bind(&filter.project).fetch_all(&state.db).await?;
    Ok(Json(rows.into_iter().map(LanguageRow::summary).collect()))
}

pub async fn language(
    State(state): State<AppState>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<LanguageDetail>> {
    let row = store::owned_language(&state, &user, &id).await?;
    let package = store::package(&state, &row.revision).await?;
    Ok(Json(LanguageDetail {
        summary: row.summary(),
        recipe: package.recipe.clone(),
        grammar: package.grammar.clone(),
        inventory: package.inventory.clone(),
        stages: package.stages.clone(),
        examples: package.examples.iter().take(8).cloned().collect(),
        limitations: package.limitations.clone(),
    }))
}

async fn enqueue(
    state: &AppState,
    user: &User,
    project: &str,
    recipe: &Recipe,
    language: Option<&str>,
    base: Option<&str>,
) -> Result<Job> {
    recipe.validate()?;
    store::owned_project(state, user, project).await?;
    let id = random_token()?;
    let result = sqlx::query("INSERT INTO jobs(id,owner_id,project_id,language_id,base_revision,recipe) SELECT ?,?,?,?,?,? WHERE (SELECT count(*) FROM jobs WHERE owner_id=? AND state IN ('queued','running','cancel_requested'))<2 AND (SELECT count(*) FROM jobs WHERE state IN ('queued','running','cancel_requested'))<16")
        .bind(&id).bind(user.id).bind(project).bind(language).bind(base).bind(serde_json::to_string(recipe)?).bind(user.id).execute(&state.db).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::Busy);
    }
    Ok(Job {
        id,
        state: "queued".into(),
        phase: "Waiting for a worker".into(),
        language_id: language.map(String::from),
        error: None,
    })
}

pub async fn generate(
    State(state): State<AppState>,
    user: User,
    headers: HeaderMap,
    Json(mut input): Json<GenerateRequest>,
) -> Result<Json<Job>> {
    user.csrf(&headers, &state)?;
    if input.recipe.seed.trim().is_empty() {
        input.recipe.seed = random_token()?;
    }
    Ok(Json(
        enqueue(&state, &user, &input.project_id, &input.recipe, None, None).await?,
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftInput {
    pub revision: String,
    pub recipe: Recipe,
}

pub async fn draft(
    State(state): State<AppState>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<DraftInput>,
) -> Result<Json<Job>> {
    user.csrf(&headers, &state)?;
    let current = store::owned_language(&state, &user, &id).await?;
    if current.revision != input.revision {
        return Err(AppError::Conflict(
            "This form is based on an older revision. Reload before editing".into(),
        ));
    }
    Ok(Json(
        enqueue(
            &state,
            &user,
            &current.project_id,
            &input.recipe,
            Some(&id),
            Some(&current.revision),
        )
        .await?,
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionInput {
    revision: String,
}

pub async fn publish(
    State(state): State<AppState>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<RevisionInput>,
) -> Result<Json<serde_json::Value>> {
    user.csrf(&headers, &state)?;
    store::owned_language(&state, &user, &id).await?;
    let result = sqlx::query("UPDATE languages SET published_revision=draft_revision,draft_revision=NULL WHERE id=? AND draft_revision=?").bind(id).bind(input.revision).execute(&state.db).await?;
    if result.rows_affected() != 1 {
        return Err(AppError::Conflict(
            "The draft changed or is already published. Reload to see its current state".into(),
        ));
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn jobs(State(state): State<AppState>, user: User) -> Result<Json<Vec<Job>>> {
    let rows: Vec<JobRow> = sqlx::query_as("SELECT id,state,phase,language_id,error FROM jobs WHERE owner_id=? ORDER BY created_at DESC,id LIMIT 30").bind(user.id).fetch_all(&state.db).await?;
    Ok(Json(rows.into_iter().map(Job::from).collect()))
}

pub async fn job(
    State(state): State<AppState>,
    user: User,
    Path(id): Path<String>,
) -> Result<Json<Job>> {
    let row: JobRow = sqlx::query_as(
        "SELECT id,state,phase,language_id,error FROM jobs WHERE id=? AND owner_id=?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row.into()))
}

pub async fn cancel(
    State(state): State<AppState>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    user.csrf(&headers, &state)?;
    let result = sqlx::query("UPDATE jobs SET state=CASE WHEN state='queued' THEN 'canceled' ELSE 'cancel_requested' END,phase='Cancellation requested',updated_at=unixepoch() WHERE id=? AND owner_id=? AND state IN ('queued','running')").bind(id).bind(user.id).execute(&state.db).await?;
    if result.rows_affected() != 1 {
        return Err(AppError::Conflict(
            "This job has already stopped or is unavailable".into(),
        ));
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(Deserialize)]
pub struct Search {
    #[serde(default)]
    q: String,
    #[serde(default)]
    offset: usize,
}
#[derive(Serialize)]
pub struct Words {
    pub entries: Vec<Entry>,
    pub total: usize,
    pub offset: usize,
}

pub async fn words(
    State(state): State<AppState>,
    user: User,
    Path(id): Path<String>,
    Query(search): Query<Search>,
) -> Result<Json<Words>> {
    if search.q.len() > 200 || search.offset > 5000 {
        return Err(AppError::Invalid("Search exceeds its limits".into()));
    }
    let package = store::owned_package(&state, &user, &id).await?;
    let query = search.q.trim().to_lowercase();
    let mut matches = Vec::new();
    for entry in &package.lexicon {
        if query.is_empty()
            || entry.gloss.to_lowercase().contains(&query)
            || entry.current()?.base.text().contains(&query)
        {
            matches.push(entry);
        }
    }
    matches.sort_by(|a, b| a.gloss.cmp(&b.gloss));
    Ok(Json(Words {
        total: matches.len(),
        offset: search.offset,
        entries: matches
            .into_iter()
            .skip(search.offset)
            .take(40)
            .cloned()
            .collect(),
    }))
}

pub async fn translate(
    State(state): State<AppState>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<TranslationRequest>,
) -> Result<Json<Translation>> {
    user.csrf(&headers, &state)?;
    let package = store::owned_package(&state, &user, &id).await?;
    let permit = state
        .interactive
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::Busy)?;
    let output = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        etyloom_engine::Runtime::new(&package)?.translate(&input.text, input.reverse)
    })
    .await
    .map_err(|e| AppError::Internal(e.into()))??;
    Ok(Json(output))
}

#[derive(Deserialize)]
pub struct LessonQuery {
    #[serde(default)]
    index: usize,
    revision: Option<String>,
}

pub async fn exercise(
    State(state): State<AppState>,
    user: User,
    Path(id): Path<String>,
    Query(query): Query<LessonQuery>,
) -> Result<Json<Exercise>> {
    let package = if let Some(revision) = query.revision {
        store::owned_revision(&state, &user, &id, &revision).await?
    } else {
        store::owned_package(&state, &user, &id).await?
    };
    let index = query.index % package.examples.len().max(1);
    let example = package
        .examples
        .get(index)
        .ok_or_else(|| AppError::Conflict("No exercises are available for this revision".into()))?;
    Ok(Json(Exercise {
        id: index,
        prompt: example.english.clone(),
        revision: package.revision.clone(),
        skill: example.skill.clone(),
    }))
}

pub async fn answer(
    State(state): State<AppState>,
    user: User,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<AnswerRequest>,
) -> Result<Json<Feedback>> {
    user.csrf(&headers, &state)?;
    let package = store::owned_revision(&state, &user, &id, &input.revision).await?;
    let permit = state
        .interactive
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::Busy)?;
    let revision = input.revision.clone();
    let exercise = input.exercise as i64;
    let feedback = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        etyloom_engine::corpus::grade(&package, input.exercise, &input.revision, &input.answer)
    })
    .await
    .map_err(|e| AppError::Internal(e.into()))??;
    sqlx::query("INSERT INTO reviews(user_id,language_id,revision,exercise,attempts,correct,streak,next_review) VALUES(?,?,?,?,1,?,?,unixepoch()+?) ON CONFLICT(user_id,language_id,revision,exercise) DO UPDATE SET attempts=attempts+1,correct=correct+excluded.correct,streak=CASE WHEN excluded.correct=1 THEN min(streak+1,5) ELSE 0 END,last_review=unixepoch(),next_review=unixepoch()+CASE WHEN excluded.correct=1 THEN 86400*(1<<min(streak,5)) ELSE 600 END")
        .bind(user.id).bind(id).bind(revision).bind(exercise).bind(i64::from(feedback.correct)).bind(i64::from(feedback.correct)).bind(if feedback.correct { 86400 } else { 600 }).execute(&state.db).await?;
    Ok(Json(feedback))
}

pub async fn export(
    State(state): State<AppState>,
    user: User,
    Path(id): Path<String>,
) -> Result<Response> {
    let package = store::owned_package(&state, &user, &id).await?;
    Ok((
        [(
            header::CONTENT_DISPOSITION,
            "attachment; filename=etyloom-language.json",
        )],
        Json(package.as_ref()),
    )
        .into_response())
}

pub async fn recipe(
    State(state): State<AppState>,
    user: User,
    Path(id): Path<String>,
) -> Result<Response> {
    let package = store::owned_package(&state, &user, &id).await?;
    Ok((
        [(
            header::CONTENT_DISPOSITION,
            "attachment; filename=etyloom-recipe.json",
        )],
        Json(&package.recipe),
    )
        .into_response())
}

#[derive(Deserialize)]
pub struct CorpusQuery {
    count: Option<usize>,
}

pub async fn corpus(
    State(state): State<AppState>,
    user: User,
    Path(id): Path<String>,
    Query(query): Query<CorpusQuery>,
) -> Result<Response> {
    let count = query.count.unwrap_or(5000);
    if count == 0 || count > 10_000 {
        return Err(AppError::Invalid(
            "Web exports support 1–10,000 scenes. Use the CLI for larger corpora".into(),
        ));
    }
    let package = store::owned_package(&state, &user, &id).await?;
    let permit = state
        .interactive
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::Busy)?;
    let bytes = tokio::task::spawn_blocking(move || -> etyloom_core::Result<Vec<u8>> {
        let _permit = permit;
        let mut output = Vec::new();
        etyloom_engine::corpus::write_corpus(&package, count, &mut output)?;
        Ok(output)
    })
    .await
    .map_err(|e| AppError::Internal(e.into()))??;
    Ok((
        [
            (header::CONTENT_TYPE, "application/x-ndjson"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=etyloom-corpus.jsonl",
            ),
        ],
        bytes,
    )
        .into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Import {
    project_id: String,
    package: Package,
}

pub async fn import(
    State(state): State<AppState>,
    user: User,
    headers: HeaderMap,
    Json(input): Json<Import>,
) -> Result<Json<serde_json::Value>> {
    user.csrf(&headers, &state)?;
    store::owned_project(&state, &user, &input.project_id).await?;
    let permit = state
        .interactive
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::Busy)?;
    let package = tokio::task::spawn_blocking(move || -> etyloom_core::Result<Package> {
        let _permit = permit;
        etyloom_engine::verify_import(&input.package)?;
        Ok(input.package)
    })
    .await
    .map_err(|e| AppError::Internal(e.into()))??;
    let id = random_token()?;
    let json = serde_json::to_string(&package)?;
    let mut transaction = state.db.begin().await?;
    store::insert_revision(&mut transaction, &package, &json).await?;
    sqlx::query("INSERT INTO languages(id,project_id,name,draft_revision) VALUES(?,?,?,?)")
        .bind(&id)
        .bind(input.project_id)
        .bind(&package.recipe.name)
        .bind(&package.revision)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("INSERT INTO language_revisions(language_id,revision) VALUES(?,?)")
        .bind(&id)
        .bind(&package.revision)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(Json(serde_json::json!({"id": id})))
}
