use etyloom_core::ApiError;
use serde::{Serialize, de::DeserializeOwned};

pub async fn get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    #[cfg(feature = "hydrate")]
    {
        let response = gloo_net::http::Request::get(path)
            .send()
            .await
            .map_err(|e| format!("The server could not be reached: {e}"))?;
        decode(response).await
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = path;
        Err("Client requests run after hydration".into())
    }
}

pub async fn post<T: DeserializeOwned, B: Serialize>(
    path: &str,
    body: &B,
    csrf: &str,
) -> Result<T, String> {
    #[cfg(feature = "hydrate")]
    {
        let response = gloo_net::http::Request::post(path)
            .header("x-csrf-token", csrf)
            .json(body)
            .map_err(|e| format!("The request could not be encoded: {e}"))?
            .send()
            .await
            .map_err(|e| format!("The server could not be reached: {e}"))?;
        decode(response).await
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = (path, body, csrf);
        Err("Client requests run after hydration".into())
    }
}

#[cfg(feature = "hydrate")]
async fn decode<T: DeserializeOwned>(response: gloo_net::http::Response) -> Result<T, String> {
    if response.status() == 401 {
        return Err("Your session has expired. Sign in again; saved work is unchanged.".into());
    }
    if !response.ok() {
        let status = response.status();
        return match response.json::<ApiError>().await {
            Ok(error) => Err(match error.request_id {
                Some(id) => format!("{} [reference {id}]", error.error),
                None => error.error,
            }),
            Err(_) => Err(format!(
                "The server returned HTTP {status}. Check your connection and retry."
            )),
        };
    }
    response
        .json()
        .await
        .map_err(|e| format!("The server response could not be read: {e}"))
}

pub async fn pause(milliseconds: u32) {
    #[cfg(feature = "hydrate")]
    gloo_timers::future::TimeoutFuture::new(milliseconds).await;
    #[cfg(not(feature = "hydrate"))]
    let _ = milliseconds;
}

pub fn navigate(path: &str) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        web_sys::window()
            .ok_or("Browser window is unavailable")?
            .location()
            .set_href(path)
            .map_err(|_| "Navigation failed".into())
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = path;
        Err("Navigation requires a browser".into())
    }
}

pub fn theme(toggle: bool) -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let window = web_sys::window().ok_or("Browser window is unavailable")?;
        let root = window
            .document()
            .and_then(|d| d.document_element())
            .ok_or("The document is unavailable")?;
        let current = root
            .get_attribute("data-theme")
            .unwrap_or_else(|| "light".into());
        if !toggle {
            return Ok(current);
        }
        let next = if current == "dark" { "light" } else { "dark" };
        root.set_attribute("data-theme", next)
            .map_err(|_| "Theme could not be applied")?;
        if let Some(storage) = window
            .local_storage()
            .map_err(|_| "Theme changed, but browser storage is blocked")?
        {
            storage
                .set_item("etyloom.theme", next)
                .map_err(|_| "Theme changed, but the preference could not be saved")?;
        }
        Ok(next.into())
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = toggle;
        Ok("light".into())
    }
}

pub fn encode_query(input: &str) -> String {
    input
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}
