use anyhow::{Context, Result, bail};
use std::{env, net::SocketAddr, path::PathBuf};
use url::Url;

#[derive(Clone)]
pub struct Config {
    pub base_url: Url,
    pub address: SocketAddr,
    pub database_url: String,
    pub site_root: PathBuf,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub development: bool,
    pub workers: usize,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let development = env::var("ETYLOOM_DEV_AUTH").as_deref() == Ok("true");
        let address: SocketAddr = env::var("LEPTOS_SITE_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3000".into())
            .parse()
            .context("Invalid LEPTOS_SITE_ADDR")?;
        let base_url =
            Url::parse(&env::var("BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".into()))
                .context("Invalid BASE_URL")?;
        if base_url.cannot_be_a_base()
            || base_url.path() != "/"
            || base_url.query().is_some()
            || base_url.fragment().is_some()
            || !base_url.username().is_empty()
            || base_url.password().is_some()
        {
            bail!("BASE_URL must be an origin without a path, credentials, query or fragment");
        }
        if development {
            if !cfg!(debug_assertions)
                || !address.ip().is_loopback()
                || !matches!(
                    base_url.host_str(),
                    Some("localhost" | "127.0.0.1" | "[::1]")
                )
            {
                bail!(
                    "Development authentication requires a debug build and a loopback-only listener and BASE_URL"
                );
            }
        } else if base_url.scheme() != "https" {
            bail!("Production BASE_URL must use HTTPS");
        }
        let issuer = env::var("OIDC_ISSUER_URL").unwrap_or_default();
        let client_id = env::var("OIDC_CLIENT_ID").unwrap_or_default();
        let client_secret = match env::var("OIDC_CLIENT_SECRET_FILE") {
            Ok(path) => std::fs::read_to_string(path)
                .context("Could not read OIDC_CLIENT_SECRET_FILE")?
                .trim()
                .to_owned(),
            Err(env::VarError::NotPresent) => env::var("OIDC_CLIENT_SECRET").unwrap_or_default(),
            Err(error) => return Err(error).context("Invalid secret-file environment value"),
        };
        if !development {
            if Url::parse(&issuer)
                .context("Set OIDC_ISSUER_URL to your Pocket ID issuer")?
                .scheme()
                != "https"
            {
                bail!("OIDC issuer must use HTTPS");
            }
            if client_id.is_empty() || client_secret.is_empty() {
                bail!(
                    "OIDC_CLIENT_ID and OIDC_CLIENT_SECRET or OIDC_CLIENT_SECRET_FILE are required"
                );
            }
        }
        let workers: usize = env::var("ETYLOOM_WORKERS")
            .unwrap_or_else(|_| "1".into())
            .parse()
            .context("Invalid ETYLOOM_WORKERS")?;
        if !(1..=4).contains(&workers) {
            bail!("ETYLOOM_WORKERS must be 1–4");
        }
        Ok(Self {
            address,
            base_url,
            issuer,
            client_id,
            client_secret,
            development,
            workers,
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://etyloom.db".into()),
            site_root: env::var("LEPTOS_SITE_ROOT")
                .unwrap_or_else(|_| "target/site".into())
                .into(),
        })
    }

    pub fn origin(&self) -> String {
        self.base_url.origin().ascii_serialization()
    }
    pub fn callback(&self) -> String {
        format!("{}/auth/callback", self.origin())
    }
    pub fn secure(&self) -> bool {
        self.base_url.scheme() == "https"
    }
}
