#![cfg(feature = "ssr")]

use axum::{body::{Body, to_bytes}, http::{Request, StatusCode}, Router};
use etyloom_core::{GenerateRequest, Job, Package, Recipe, TranslationRequest};
use etyloom_web::server::{self, AppState, config::Config};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tempfile::TempDir;
use tokio::sync::{RwLock, Semaphore};
use tower::ServiceExt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct Fixture { app: Router, state: AppState, token: String, csrf: String, _directory: TempDir }

impl Fixture {
    async fn new() -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let database_url = format!("sqlite://{}",directory.path().join("test.db").display());
        let db = server::connect(&database_url).await?;
        let config = Config {
            base_url: "http://127.0.0.1:3000".parse()?, address: "127.0.0.1:3000".parse()?, database_url,
            site_root: directory.path().into(), issuer: "https://identity.example".into(), client_id: "tests".into(), client_secret: "not-a-real-secret".into(), development: true, workers: 1,
        };
        let state = AppState { config: Arc::new(config),db,http:reqwest::Client::new(),cache:Arc::new(RwLock::new(BTreeMap::new())),interactive:Arc::new(Semaphore::new(2)) };
        let token = "a".repeat(64);
        let csrf = "b".repeat(64);
        sqlx::query("INSERT INTO users(id,issuer,subject,name) VALUES(1,'test','owner','Owner'),(2,'test','other','Other')").execute(&state.db).await?;
        sqlx::query("INSERT INTO sessions(token_hash,user_id,csrf,expires_at) VALUES(?,1,?,unixepoch()+3600)").bind(blake3::hash(token.as_bytes()).to_hex().to_string()).bind(&csrf).execute(&state.db).await?;
        sqlx::query("INSERT INTO projects(id,owner_id,name) VALUES('ours',1,'Our project'),('theirs',2,'Other project')").execute(&state.db).await?;
        let app = server::api_router().with_state(state.clone());
        Ok(Self { app,state,token,csrf,_directory:directory })
    }

    async fn request(&self, method: &str, path: &str, body: Value, signed: bool, csrf: bool) -> Result<(StatusCode, Value)> {
        let mut request = Request::builder().method(method).uri(path).header("content-type","application/json");
        if signed { request = request.header("cookie",format!("etyloom_session={}",self.token)); }
        if csrf { request = request.header("x-csrf-token",&self.csrf); }
        let response = self.app.clone().oneshot(request.body(Body::from(serde_json::to_vec(&body)?))?).await?;
        let status = response.status();
        let bytes = to_bytes(response.into_body(),16*1024*1024).await?;
        Ok((status,serde_json::from_slice(&bytes)?))
    }

    async fn generated(&self) -> Result<String> {
        let body = serde_json::to_value(GenerateRequest { project_id:"ours".into(),recipe:Recipe { name:"Test language".into(),seed:"api-tests".into(),lexicon_size:128,..Recipe::default() } })?;
        let (status,value) = self.request("POST","/api/generate",body,true,true).await?;
        assert_eq!(status,StatusCode::OK);
        let job: Job = serde_json::from_value(value)?;
        let worker = tokio::spawn(server::jobs::worker(self.state.clone()));
        for _ in 0..100 {
            let (_,value) = self.request("GET",&format!("/api/jobs/{}",job.id),Value::Null,true,false).await?;
            let current: Job = serde_json::from_value(value)?;
            if current.state == "completed" {
                worker.abort();
                return current.language_id.ok_or_else(|| "Completed job has no language".into());
            }
            if current.state == "failed" { worker.abort(); return Err(format!("Generation failed: {:?}",current.error).into()); }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        worker.abort();
        Err("Job did not complete within the test budget".into())
    }
}

#[tokio::test]
async fn authentication_csrf_and_project_isolation() -> Result<()> {
    let fixture = Fixture::new().await?;
    assert_eq!(fixture.request("GET","/api/projects",Value::Null,false,false).await?.0,StatusCode::UNAUTHORIZED);
    assert_eq!(fixture.request("POST","/api/projects",json!({"name":"Blocked"}),true,false).await?.0,StatusCode::FORBIDDEN);
    let (_,projects) = fixture.request("GET","/api/projects",Value::Null,true,false).await?;
    assert_eq!(projects.as_array().map(Vec::len),Some(1));
    assert_eq!(projects[0]["id"],"ours");
    let body = serde_json::to_value(GenerateRequest { project_id:"theirs".into(),recipe:Recipe::default() })?;
    assert_eq!(fixture.request("POST","/api/generate",body,true,true).await?.0,StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn generation_translation_publication_and_export() -> Result<()> {
    let fixture = Fixture::new().await?;
    let id = fixture.generated().await?;
    let (_,value) = fixture.request("GET",&format!("/api/languages/{id}/export"),Value::Null,true,false).await?;
    let package: Package = serde_json::from_value(value)?;
    etyloom_engine::verify_import(&package)?;
    let body = serde_json::to_value(TranslationRequest { text:"I see the river".into(),reverse:false })?;
    let (status,translated) = fixture.request("POST",&format!("/api/languages/{id}/translate"),body,true,true).await?;
    assert_eq!(status,StatusCode::OK);
    assert!(translated["output"].as_str().is_some_and(|s| !s.is_empty()));
    let publish = json!({"revision":package.revision});
    assert_eq!(fixture.request("POST",&format!("/api/languages/{id}/publish"),publish.clone(),true,true).await?.0,StatusCode::OK);
    assert_eq!(fixture.request("POST",&format!("/api/languages/{id}/publish"),publish,true,true).await?.0,StatusCode::CONFLICT);
    let (_,detail) = fixture.request("GET",&format!("/api/languages/{id}"),Value::Null,true,false).await?;
    assert_eq!(detail["summary"]["status"],"published");
    assert_eq!(detail["summary"]["revision"],package.revision);
    Ok(())
}

#[tokio::test]
async fn package_import_checks_checksum_and_ownership() -> Result<()> {
    let fixture = Fixture::new().await?;
    let mut package = etyloom_engine::generate(Recipe { lexicon_size:128,..Recipe::default() })?;
    package.recipe.name = "Tampered".into();
    let (status,_) = fixture.request("POST","/api/import",json!({"project_id":"ours","package":package}),true,true).await?;
    assert_eq!(status,StatusCode::UNPROCESSABLE_ENTITY);
    let (_,languages) = fixture.request("GET","/api/languages",Value::Null,true,false).await?;
    assert_eq!(languages.as_array().map(Vec::len),Some(0));
    Ok(())
}

#[tokio::test]
async fn queued_job_can_be_canceled_without_creating_language() -> Result<()> {
    let fixture = Fixture::new().await?;
    let body = serde_json::to_value(GenerateRequest { project_id:"ours".into(),recipe:Recipe::default() })?;
    let (_,value) = fixture.request("POST","/api/generate",body,true,true).await?;
    let job: Job = serde_json::from_value(value)?;
    assert_eq!(fixture.request("POST",&format!("/api/jobs/{}/cancel",job.id),json!({}),true,true).await?.0,StatusCode::OK);
    let (_,value) = fixture.request("GET",&format!("/api/jobs/{}",job.id),Value::Null,true,false).await?;
    assert_eq!(value["state"],"canceled");
    Ok(())
}
