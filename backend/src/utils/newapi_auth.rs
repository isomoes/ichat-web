use anyhow::Context as _;
use dotenv::var;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

const SELF_ID_FIELDS: &[&str] = &["id", "user_id"];
const SELF_USERNAME_FIELDS: &[&str] = &["username", "name", "email"];
const TOKEN_FIELDS: &[&str] = &["token", "access_token", "key"];
const REASON_FIELDS: &[&str] = &["message", "msg", "reason", "error"];

#[derive(Debug, Clone)]
pub struct ExternalIdentity {
    pub external_user_id: String,
    pub username: String,
}

#[derive(Clone)]
pub struct AuthenticatedExternalUser {
    pub identity: ExternalIdentity,
    session: Client,
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
        let session = self.build_session_client()?;
        let auth = self
            .authenticate(
                &session,
                "login",
                json!({
                    "username": username,
                    "password": password,
                }),
            )
            .await?;

        let identity = self.fetch_self(&session, auth).await?;

        Ok(AuthenticatedExternalUser { identity, session }.identity)
    }

    pub async fn register(
        &self,
        username: &str,
        password: &str,
        email: Option<&str>,
        verification_code: Option<&str>,
        aff_code: Option<&str>,
    ) -> anyhow::Result<ExternalIdentity> {
        let session = self.build_session_client()?;
        let auth = self
            .authenticate(
                &session,
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

        let identity = self.fetch_self(&session, auth).await?;

        Ok(AuthenticatedExternalUser { identity, session }.identity)
    }

    pub async fn login_with_session(
        &self,
        username: &str,
        password: &str,
    ) -> anyhow::Result<AuthenticatedExternalUser> {
        let session = self.build_session_client()?;
        let auth = self
            .authenticate(
                &session,
                "login",
                json!({
                    "username": username,
                    "password": password,
                }),
            )
            .await?;

        let identity = self.fetch_self(&session, auth).await?;

        Ok(AuthenticatedExternalUser { identity, session })
    }

    pub async fn register_with_session(
        &self,
        username: &str,
        password: &str,
        email: Option<&str>,
        verification_code: Option<&str>,
        aff_code: Option<&str>,
    ) -> anyhow::Result<AuthenticatedExternalUser> {
        let session = self.build_session_client()?;
        let auth = self
            .authenticate(
                &session,
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

        let identity = self.fetch_self(&session, auth).await?;

        Ok(AuthenticatedExternalUser { identity, session })
    }

    pub async fn ensure_user_api_key(
        &self,
        user: &AuthenticatedExternalUser,
    ) -> anyhow::Result<String> {
        let user_id = &user.identity.external_user_id;
        let keys = self
            .fetch_all_keys_with_session(&user.session, user_id)
            .await?;

        if let Some(key) = keys.into_iter().next() {
            return Ok(key);
        }

        let created = self
            .create_one_key_with_session(&user.session, user_id)
            .await?;
        if !created {
            anyhow::bail!("no existing New API key and automatic creation failed");
        }

        self.fetch_all_keys_with_session(&user.session, user_id)
            .await?
            .into_iter()
            .next()
            .context("New API key creation succeeded but no key was returned")
    }

    fn build_session_client(&self) -> anyhow::Result<Client> {
        Client::builder()
            .cookie_store(true)
            .build()
            .context("failed to build New API session client")
    }

    async fn authenticate(
        &self,
        session: &Client,
        path: &str,
        body: Value,
    ) -> anyhow::Result<UpstreamAuth> {
        let response = session
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
        let external_user_id = extract_external_user_id(&response_text);

        Ok(UpstreamAuth {
            bearer_token,
            external_user_id,
            has_session_cookie,
        })
    }

    async fn fetch_self(
        &self,
        session: &Client,
        auth: UpstreamAuth,
    ) -> anyhow::Result<ExternalIdentity> {
        let mut request = session.get(format!("{}/api/user/self", self.base_url));

        if let Some(service_bearer) = &self.service_bearer {
            request = request.header("X-NewApi-Service-Bearer", service_bearer);
        }

        if let Some(external_user_id) = &auth.external_user_id {
            request = request.header(&self.user_header, external_user_id);
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
        let external_user_id = extract_stringish_field(object, SELF_ID_FIELDS)
            .context("New API self response missing id")?;
        let username = extract_stringish_field(object, SELF_USERNAME_FIELDS)
            .context("New API self response missing username")?;

        Ok(ExternalIdentity {
            external_user_id,
            username,
        })
    }

    async fn fetch_all_keys_with_session(
        &self,
        session: &Client,
        user_id: &str,
    ) -> anyhow::Result<Vec<String>> {
        let token_response = session
            .get(format!("{}/api/token/?p=1&page_size=100", self.base_url))
            .headers(self.token_headers(user_id, "application/json")?)
            .send()
            .await
            .context("failed to query New API token list")?;
        let status = token_response.status();
        let token_data = token_response
            .json::<Value>()
            .await
            .context("failed to decode New API token list")?;

        if !status.is_success() {
            anyhow::bail!(extract_reason_value(&token_data).unwrap_or_else(|| {
                format!("New API token list failed with status {}", status)
            }));
        }

        let token_items = token_data
            .get("data")
            .and_then(Value::as_object)
            .and_then(|data| data.get("items"))
            .and_then(Value::as_array)
            .context("New API token list missing items")?;

        let mut all_keys = Vec::with_capacity(token_items.len());
        for item in token_items {
            let key_id =
                extract_stringish_field(item, &["id"]).context("New API token missing id")?;
            let key_response = session
                .post(format!("{}/api/token/{}/key", self.base_url, key_id))
                .headers(self.token_headers(user_id, "application/json, text/plain, */*")?)
                .send()
                .await
                .with_context(|| format!("failed to query New API key for token {}", key_id))?;
            let status = key_response.status();
            let key_data = key_response
                .json::<Value>()
                .await
                .with_context(|| format!("failed to decode New API key for token {}", key_id))?;

            if !status.is_success() {
                anyhow::bail!(extract_reason_value(&key_data).unwrap_or_else(|| {
                    format!("New API token key lookup failed with status {}", status)
                }));
            }

            let real_key = key_data
                .get("data")
                .and_then(Value::as_object)
                .and_then(|data| data.get("key"))
                .and_then(Value::as_str)
                .filter(|key| !key.is_empty())
                .context("New API token key response missing key")?;

            all_keys.push(format!("sk-{real_key}"));
        }

        Ok(all_keys)
    }

    async fn create_one_key_with_session(
        &self,
        session: &Client,
        user_id: &str,
    ) -> anyhow::Result<bool> {
        let create_response = session
            .post(format!("{}/api/token/", self.base_url))
            .headers(self.create_headers(user_id)?)
            .json(&json!({
                "name": random_token_name(),
                "remain_quota": 0,
                "expired_time": -1,
                "unlimited_quota": true,
                "model_limits_enabled": false,
                "model_limits": "",
                "allow_ips": "",
                "group": "",
                "cross_group_retry": false
            }))
            .send()
            .await
            .context("failed to create New API token")?;
        let status = create_response.status();
        let create_data = create_response
            .json::<Value>()
            .await
            .context("failed to decode New API token creation response")?;

        if !status.is_success() {
            anyhow::bail!(extract_reason_value(&create_data).unwrap_or_else(|| {
                format!("New API token creation failed with status {}", status)
            }));
        }

        Ok(create_data
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    fn token_headers(
        &self,
        user_id: &str,
        accept: &str,
    ) -> anyhow::Result<reqwest::header::HeaderMap> {
        use reqwest::header::{ACCEPT, HeaderMap, HeaderName, HeaderValue, ORIGIN, REFERER, USER_AGENT};

        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_str(accept)?);
        headers.insert(
            HeaderName::from_bytes(self.user_header.as_bytes())?,
            HeaderValue::from_str(user_id)?,
        );
        headers.insert(
            REFERER,
            HeaderValue::from_str(&format!("{}/console/token", self.base_url))?,
        );
        headers.insert(ORIGIN, HeaderValue::from_str(&self.base_url)?);
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0"));
        Ok(headers)
    }

    fn create_headers(&self, user_id: &str) -> anyhow::Result<reqwest::header::HeaderMap> {
        use reqwest::header::{CONTENT_TYPE, HeaderValue};

        let mut headers = self.token_headers(user_id, "application/json, text/plain, */*")?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        Ok(headers)
    }
}

fn random_token_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("ichat-{nanos}")
}

#[derive(Debug, Clone)]
struct UpstreamAuth {
    bearer_token: Option<String>,
    external_user_id: Option<String>,
    #[allow(dead_code)]
    has_session_cookie: bool,
}

fn extract_bearer_token(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    let object = value.get("data").unwrap_or(&value);

    extract_stringish_field(object, TOKEN_FIELDS)
}

fn extract_external_user_id(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    let object = value.get("data").unwrap_or(&value);

    extract_stringish_field(object, SELF_ID_FIELDS)
}

fn extract_reason(text: &str) -> Option<String> {
    let value: Value = serde_json::from_str(text).ok()?;
    extract_reason_value(&value)
}

fn extract_reason_value(value: &Value) -> Option<String> {
    extract_stringish_field(value, REASON_FIELDS).or_else(|| {
        value
            .get("data")
            .and_then(|data| extract_stringish_field(data, REASON_FIELDS))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_external_user_id_from_data_payload() {
        let body = r#"{
            "data": {
                "display_name": "Root User",
                "group": "default",
                "id": 1,
                "role": 100,
                "status": 1,
                "username": "isomoes"
            },
            "message": "",
            "success": true
        }"#;

        assert_eq!(extract_external_user_id(body).as_deref(), Some("1"));
    }
}
