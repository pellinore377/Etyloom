use super::{AppState, error::{AppError, Result}};
use anyhow::{Context, anyhow};
use axum::{Json, extract::{FromRequestParts, Query, State}, http::{HeaderMap, header, request::Parts}, response::{IntoResponse, Redirect, Response}};
use etyloom_core::SessionInfo;
use openidconnect::{core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata}, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse};
use serde::Deserialize;
use subtle::ConstantTimeEq;

type Client = CoreClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointMaybeSet, EndpointMaybeSet>;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct User { pub id: i64, pub name: String, pub csrf: String, pub token_hash: String }

impl FromRequestParts<AppState> for User {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        current(state, &parts.headers).await?.ok_or(AppError::Unauthorized)
    }
}

impl User {
    pub fn csrf(&self, headers: &HeaderMap, state: &AppState) -> Result<()> {
        let origin = headers.get(header::ORIGIN).and_then(|h| h.to_str().ok());
        if origin.is_some_and(|value| value != state.config.origin()) { return Err(AppError::Forbidden); }
        let token = headers.get("x-csrf-token").and_then(|h| h.to_str().ok()).ok_or(AppError::Forbidden)?;
        if !bool::from(token.as_bytes().ct_eq(self.csrf.as_bytes())) { return Err(AppError::Forbidden); }
        Ok(())
    }
}

pub fn random_token() -> Result<String> {
    let mut bytes = [0; 32];
    getrandom::getrandom(&mut bytes).map_err(|e| AppError::Internal(anyhow!("Operating-system randomness failed: {e}")))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn digest(token: &str) -> String { blake3::hash(token.as_bytes()).to_hex().to_string() }

fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut found = None;
    for value in headers.get_all(header::COOKIE) {
        let value = value.to_str().ok()?;
        for part in value.split(';') {
            let Some((key, value)) = part.trim().split_once('=') else { continue; };
            if key == name {
                if found.is_some() { return None; }
                found = Some(value);
            }
        }
    }
    found
}

fn set_cookie(state: &AppState, name: &str, value: &str, seconds: usize) -> Result<header::HeaderValue> {
    format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={seconds}{}", if state.config.secure() { "; Secure" } else { "" }).parse().map_err(|e| AppError::Internal(anyhow!("Cookie serialization failed: {e}")))
}

async fn current(state: &AppState, headers: &HeaderMap) -> Result<Option<User>> {
    let Some(token) = cookie(headers, "etyloom_session") else { return Ok(None); };
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) { return Ok(None); }
    Ok(sqlx::query_as("SELECT u.id, u.name, s.csrf, s.token_hash FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.token_hash=? AND s.expires_at>unixepoch()")
        .bind(digest(token)).fetch_optional(&state.db).await?)
}

pub async fn session(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<SessionInfo>> {
    let user = current(&state, &headers).await?;
    Ok(Json(SessionInfo { authenticated: user.is_some(), display_name: user.as_ref().map(|u| u.name.clone()).unwrap_or_default(), csrf: user.as_ref().map(|u| u.csrf.clone()).unwrap_or_default(), development: state.config.development }))
}

async fn client(state: &AppState) -> Result<Client> {
    let metadata = CoreProviderMetadata::discover_async(IssuerUrl::new(state.config.issuer.clone()).context("Invalid issuer")?, &state.http).await.context("Pocket ID discovery failed")?;
    Ok(CoreClient::from_provider_metadata(metadata, ClientId::new(state.config.client_id.clone()), Some(ClientSecret::new(state.config.client_secret.clone())))
        .set_redirect_uri(RedirectUrl::new(state.config.callback()).context("Invalid callback URL")?))
}

pub async fn login(State(state): State<AppState>, headers: HeaderMap) -> Result<Response> {
    if state.config.development { return dev_login(State(state)).await; }
    sqlx::query("DELETE FROM logins WHERE expires_at<=unixepoch()").execute(&state.db).await?;
    if let Some(old) = cookie(&headers, "etyloom_login") { sqlx::query("DELETE FROM logins WHERE token_hash=?").bind(digest(old)).execute(&state.db).await?; }
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM logins").fetch_one(&state.db).await?;
    if count >= 1000 { return Err(AppError::Busy); }
    let client = client(&state).await?;
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let (url, csrf, nonce) = client.authorize_url(CoreAuthenticationFlow::AuthorizationCode, CsrfToken::new_random, Nonce::new_random)
        .add_scope(Scope::new("profile".into())).set_pkce_challenge(challenge).url();
    let token = random_token()?;
    sqlx::query("INSERT INTO logins(token_hash,state,nonce,pkce,expires_at) VALUES(?,?,?,?,unixepoch()+600)")
        .bind(digest(&token)).bind(csrf.secret()).bind(nonce.secret()).bind(verifier.secret()).execute(&state.db).await?;
    let mut response = Redirect::to(url.as_str()).into_response();
    response.headers_mut().append(header::SET_COOKIE, set_cookie(&state, "etyloom_login", &token, 600)?);
    Ok(response)
}

#[derive(Deserialize)]
pub struct Callback { code: Option<String>, state: Option<String>, error: Option<String> }

pub async fn callback(State(state): State<AppState>, headers: HeaderMap, Query(query): Query<Callback>) -> Result<Response> {
    if state.config.development { return Err(AppError::NotFound); }
    let token = cookie(&headers, "etyloom_login").ok_or(AppError::Unauthorized)?;
    let pending: Option<(String, String, String)> = sqlx::query_as("DELETE FROM logins WHERE token_hash=? AND expires_at>unixepoch() RETURNING state,nonce,pkce")
        .bind(digest(token)).fetch_optional(&state.db).await?;
    let (expected, nonce, pkce) = pending.ok_or(AppError::Unauthorized)?;
    let supplied = query.state.ok_or(AppError::Unauthorized)?;
    if !bool::from(supplied.as_bytes().ct_eq(expected.as_bytes())) || query.error.is_some() { return Err(AppError::Unauthorized); }
    let client = client(&state).await?;
    let tokens = client.exchange_code(AuthorizationCode::new(query.code.ok_or(AppError::Unauthorized)?))
        .context("Token endpoint unavailable")?.set_pkce_verifier(PkceCodeVerifier::new(pkce)).request_async(&state.http).await.context("OIDC token exchange failed")?;
    let id = tokens.id_token().ok_or_else(|| AppError::Internal(anyhow!("Identity provider returned no ID token")))?;
    let claims = id.claims(&client.id_token_verifier(), &Nonce::new(nonce)).context("OIDC token validation failed")?;
    let name = claims.preferred_username().map(|n| n.as_str()).unwrap_or("Etyloom member");
    let (user_id,): (i64,) = sqlx::query_as("INSERT INTO users(issuer,subject,name) VALUES(?,?,?) ON CONFLICT(issuer,subject) DO UPDATE SET name=excluded.name RETURNING id")
        .bind(&state.config.issuer).bind(claims.subject().as_str()).bind(name).fetch_one(&state.db).await?;
    let mut response = establish(&state, user_id, &headers).await?;
    response.headers_mut().append(header::SET_COOKIE, set_cookie(&state, "etyloom_login", "", 0)?);
    Ok(response)
}

pub async fn dev_login(State(state): State<AppState>) -> Result<Response> {
    if !state.config.development || !cfg!(debug_assertions) { return Err(AppError::NotFound); }
    let (id,): (i64,) = sqlx::query_as("INSERT INTO users(issuer,subject,name) VALUES('local-development','developer','Local developer') ON CONFLICT(issuer,subject) DO UPDATE SET name=excluded.name RETURNING id").fetch_one(&state.db).await?;
    establish(&state, id, &HeaderMap::new()).await
}

async fn establish(state: &AppState, user: i64, headers: &HeaderMap) -> Result<Response> {
    let token = random_token()?;
    let csrf = random_token()?;
    let mut transaction = state.db.begin().await?;
    if let Some(old) = cookie(headers, "etyloom_session") { sqlx::query("DELETE FROM sessions WHERE token_hash=?").bind(digest(old)).execute(&mut *transaction).await?; }
    sqlx::query("INSERT INTO sessions(token_hash,user_id,csrf,expires_at) VALUES(?,?,?,unixepoch()+1209600)").bind(digest(&token)).bind(user).bind(csrf).execute(&mut *transaction).await?;
    transaction.commit().await?;
    let mut response = Redirect::to("/").into_response();
    response.headers_mut().append(header::SET_COOKIE, set_cookie(state, "etyloom_session", &token, 1_209_600)?);
    Ok(response)
}

pub async fn logout(State(state): State<AppState>, user: User, headers: HeaderMap) -> Result<Response> {
    user.csrf(&headers, &state)?;
    sqlx::query("DELETE FROM sessions WHERE token_hash=?").bind(user.token_hash).execute(&state.db).await?;
    let mut response = Json(serde_json::json!({"ok": true})).into_response();
    response.headers_mut().append(header::SET_COOKIE, set_cookie(&state, "etyloom_session", "", 0)?);
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_session_cookie_is_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, header::HeaderValue::from_static("etyloom_session=a; etyloom_session=b"));
        assert!(cookie(&headers, "etyloom_session").is_none());
    }
    #[test]
    fn token_has_full_entropy_encoding() -> Result<()> {
        let a = random_token()?;
        let b = random_token()?;
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        Ok(())
    }
}
