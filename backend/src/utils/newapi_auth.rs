use anyhow::Context as _;
use dotenv::var;
use reqwest::Client;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct ExternalIdentity {
    pub external_user_id: String,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct NewApiAuthClient {
    pub base_url: String,
    pub user_header: String,
    pub service_bearer: Option<String>,
    pub http: Client,
}

impl NewApiAuthClient {
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let Ok(base_url) = var("NEWAPI_AUTH_BASE") else {
            return Ok(None);
        };

        let http = Client::builder()
            .cookie_store(true)
            .build()
            .context("failed to build New API auth client")?;

        Ok(Some(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            user_header: var("NEWAPI_AUTH_USER_HEADER")
                .unwrap_or_else(|_| "New-Api-User".to_owned()),
            service_bearer: var("NEWAPI_AUTH_BEARER").ok(),
            http,
        }))
    }

    pub fn is_enabled(&self) -> bool {
        true
    }

    pub async fn login(&self, username: &str, password: &str) -> anyhow::Result<ExternalIdentity> {
        let auth = self
            .authenticate(
                "login",
                json!({
                    "username": username,
                    "password": password,
                }),
            )
            .await?;

        self.fetch_self(auth).await
    }

    pub async fn register(
        &self,
        username: &str,
        password: &str,
        email: Option<&str>,
        verification_code: Option<&str>,
        aff_code: Option<&str>,
    ) -> anyhow::Result<ExternalIdentity> {
        let auth = self
            .authenticate(
                "register",
                json!({
                    "username": username,
                    "password": password,
                    "email": email,
                    "verification_code": verification_code,
                    "aff_code": aff_code,
                }),
            )
            .await?;

        self.fetch_self(auth).await
    }

    async fn authenticate(&self, path: &str, body: Value) -> anyhow::Result<UpstreamAuth> {
        let response = self
            .http
            .post(format!("{}/api/user/{}", self.base_url, path))
            .json(&body)
            .send()
            .await
            .with_context(|| format!("failed to call New API {}", path))?;

        let has_session_cookie = response
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .next()
            .is_some();
        let status = response.status();
        let response_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            anyhow::bail!(
                extract_reason(&response_text).unwrap_or_else(|| {
                    format!("New API {} failed with status {}", path, status)
                })
            );
        }

        let bearer_token = extract_bearer_token(&response_text);

        Ok(UpstreamAuth {
            bearer_token,
            has_session_cookie,
        })
    }

    async fn fetch_self(&self, auth: UpstreamAuth) -> anyhow::Result<ExternalIdentity> {
        let mut request = self.http.get(format!("{}/api/user/self", self.base_url));

        if let Some(service_bearer) = &self.service_bearer {
            request = request.header("X-NewApi-Service-Bearer", service_bearer);
        }

        if let Some(bearer_token) = &auth.bearer_token {
            request = request.bearer_auth(bearer_token);
        }

        let response = request
            .send()
            .await
            .context("failed to call New API self endpoint")?;
        let status = response.status();
        let value = response.json::<Value>().await.unwrap_or(Value::Null);

        if !status.is_success() {
            anyhow::bail!(
                extract_reason_value(&value).unwrap_or_else(|| format!(
                    "New API self lookup failed with status {}",
                    status
                ))
            );
        }

        let object = value.get("data").unwrap_or(&value);
        let external_user_id = extract_stringish_field(object, &["id", "user_id"])
            .context("New API self response missing id")?;
        let username = extract_stringish_field(object, &["username", "name", "email"])
            .context("New API self response missing username")?;

        Ok(ExternalIdentity {
            external_user_id,
            username,
        })
    }
}

#[derive(Debug, Clone)]
struct UpstreamAuth {
    bearer_token: Option<String>,
    #[allow(dead_code)]
    has_session_cookie: bool,
}

fn extract_bearer_token(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    let object = value.get("data").unwrap_or(&value);

    extract_stringish_field(object, &["token", "access_token", "key"])
}

fn extract_reason(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    extract_reason_value(&value)
}

fn extract_reason_value(value: &Value) -> Option<String> {
    extract_stringish_field(value, &["message", "msg", "reason", "error"]).or_else(|| {
        value
            .get("data")
            .and_then(|data| extract_stringish_field(data, &["message", "msg", "reason", "error"]))
    })
}

fn extract_stringish_field(value: &Value, fields: &[&str]) -> Option<String> {
    for field in fields {
        let Some(item) = value.get(*field) else {
            continue;
        };

        match item {
            Value::String(text) if !text.is_empty() => return Some(text.clone()),
            Value::Number(number) => return Some(number.to_string()),
            _ => {}
        }
    }

    None
}
