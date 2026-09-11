use super::{AppState, error::{AppError, Result}, store};
use etyloom_core::{Package, Recipe};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::sync::mpsc;

#[derive(sqlx::FromRow)]
struct Work {
    id: String,
    project_id: String,
    language_id: Option<String>,
    base_revision: Option<String>,
    recipe: String,
}

pub async fn worker(state: AppState) {
    loop {
        match next(&state).await {
            Ok(Some(work)) => {
                if let Err(error) = execute(&state, &work).await {
                    let message = match &error {
                        AppError::Engine(e) => e.to_string(), AppError::Conflict(e) => e.clone(),
                        _ => "Generation could not be saved. Existing revisions are unchanged; check the server log and retry.".into(),
                    };
                    tracing::error!(job = %work.id, error = %error, "generation failed");
                    if let Err(error) = sqlx::query("UPDATE jobs SET state=CASE WHEN state='cancel_requested' THEN 'canceled' ELSE 'failed' END,error=?,phase='Job stopped',updated_at=unixepoch() WHERE id=? AND state!='completed'").bind(message).bind(&work.id).execute(&state.db).await {
                        tracing::error!(job = %work.id, %error, "failed to record job failure");
                    }
                }
            }
            Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
            Err(error) => { tracing::error!(%error, "job polling failed"); tokio::time::sleep(std::time::Duration::from_secs(3)).await; }
        }
    }
}

async fn next(state: &AppState) -> Result<Option<Work>> {
    Ok(sqlx::query_as("UPDATE jobs SET state='running',phase='Starting deterministic engine',updated_at=unixepoch() WHERE id=(SELECT id FROM jobs WHERE state='queued' ORDER BY created_at,id LIMIT 1) AND state='queued' RETURNING id,project_id,language_id,base_revision,recipe").fetch_optional(&state.db).await?)
}

async fn execute(state: &AppState, work: &Work) -> Result<()> {
    let recipe: Recipe = serde_json::from_str(&work.recipe)?;
    let canceled = Arc::new(AtomicBool::new(false));
    let cancel = canceled.clone();
    let (sender, mut receiver) = mpsc::channel::<String>(16);
    let task = tokio::task::spawn_blocking(move || {
        etyloom_engine::generate_with(recipe, |phase| sender.blocking_send(phase.to_owned()).map_err(|_| etyloom_core::Error::Canceled), || cancel.load(Ordering::Relaxed))
    });
    while let Some(phase) = receiver.recv().await {
        let (status,): (String,) = sqlx::query_as("SELECT state FROM jobs WHERE id=?").bind(&work.id).fetch_one(&state.db).await?;
        if status == "cancel_requested" { canceled.store(true, Ordering::Relaxed); }
        sqlx::query("UPDATE jobs SET phase=?,updated_at=unixepoch() WHERE id=? AND state='running'").bind(phase).bind(&work.id).execute(&state.db).await?;
    }
    let generated = task.await.map_err(|error| AppError::Internal(error.into()))?;
    if canceled.load(Ordering::Relaxed) || matches!(generated, Err(etyloom_core::Error::Canceled)) {
        sqlx::query("UPDATE jobs SET state='canceled',phase='Canceled; existing revisions preserved',updated_at=unixepoch() WHERE id=?").bind(&work.id).execute(&state.db).await?;
        return Ok(());
    }
    persist(state, work, generated?).await
}

async fn persist(state: &AppState, work: &Work, package: Package) -> Result<()> {
    let package = Arc::new(package);
    let to_encode = package.clone();
    let json = tokio::task::spawn_blocking(move || serde_json::to_string(to_encode.as_ref())).await.map_err(|e| AppError::Internal(e.into()))??;
    let mut transaction = state.db.begin().await?;
    let (status,): (String,) = sqlx::query_as("SELECT state FROM jobs WHERE id=?").bind(&work.id).fetch_one(&mut *transaction).await?;
    if status == "cancel_requested" {
        sqlx::query("UPDATE jobs SET state='canceled',phase='Canceled; existing revisions preserved' WHERE id=?").bind(&work.id).execute(&mut *transaction).await?;
        transaction.commit().await?;
        return Ok(());
    }
    store::insert_revision(&mut transaction, &package, &json).await?;
    let language_id = work.language_id.as_ref().unwrap_or(&work.id);
    if work.language_id.is_some() {
        let result = sqlx::query("UPDATE languages SET draft_revision=?,name=? WHERE id=? AND coalesce(draft_revision,published_revision)=?")
            .bind(&package.revision).bind(&package.recipe.name).bind(language_id).bind(&work.base_revision).execute(&mut *transaction).await?;
        if result.rows_affected() != 1 { return Err(AppError::Conflict("The language changed while this draft was generating. Regenerate from its latest revision".into())); }
    } else {
        sqlx::query("INSERT INTO languages(id,project_id,name,draft_revision) VALUES(?,?,?,?)").bind(language_id).bind(&work.project_id).bind(&package.recipe.name).bind(&package.revision).execute(&mut *transaction).await?;
    }
    sqlx::query("INSERT INTO language_revisions(language_id,revision,parent_revision) VALUES(?,?,?) ON CONFLICT DO NOTHING").bind(language_id).bind(&package.revision).bind(&work.base_revision).execute(&mut *transaction).await?;
    sqlx::query("UPDATE jobs SET state='completed',phase='Validated draft saved',language_id=?,error=NULL,updated_at=unixepoch() WHERE id=?").bind(language_id).bind(&work.id).execute(&mut *transaction).await?;
    transaction.commit().await?;
    let mut cache = state.cache.write().await;
    if cache.len() >= 4 { cache.pop_first(); }
    cache.insert(package.revision.clone(), package);
    tracing::info!(job = %work.id, language = %language_id, "generation saved");
    Ok(())
}
