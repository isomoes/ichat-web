use std::sync::Arc;

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use typeshare::typeshare;

use crate::{AppState, errors::*};

#[derive(Debug, Deserialize)]
#[typeshare]
pub struct VerificationReq {
    pub email: String,
}

#[derive(Debug, Serialize)]
#[typeshare]
pub struct VerificationResp {
    pub success: bool,
}

pub async fn route(
    State(app): State<Arc<AppState>>,
    Json(req): Json<VerificationReq>,
) -> JsonResult<VerificationResp> {
    let Some(newapi_auth) = &app.newapi_auth else {
        return Err(Json(Error {
            error: ErrorKind::MalformedRequest,
            reason: "External registration is not enabled".to_owned(),
        }));
    };

    let response = newapi_auth
        .http
        .get(format!("{}/api/verification", newapi_auth.base_url))
        .query(&[("email", req.email.as_str())])
        .send()
        .await
        .map_err(|error| {
            Json(Error {
                error: ErrorKind::MalformedRequest,
                reason: error.to_string(),
            })
        })?;

    let status = response.status();
    let value = response.json::<Value>().await.map_err(|error| {
        Json(Error {
            error: ErrorKind::MalformedRequest,
            reason: error.to_string(),
        })
    })?;

    if !status.is_success() {
        return Err(Json(Error {
            error: ErrorKind::MalformedRequest,
            reason: extract_reason(&value)
                .unwrap_or_else(|| format!("New API verification failed with status {}", status)),
        }));
    }

    if !value
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        return Err(Json(Error {
            error: ErrorKind::MalformedRequest,
            reason: extract_reason(&value).unwrap_or_else(|| "New API verification failed".to_owned()),
        }));
    }

    Ok(Json(VerificationResp { success: true }))
}

fn extract_reason(value: &Value) -> Option<String> {
    [value.get("message"), value.get("msg"), value.get("reason"), value.get("error")]
        .into_iter()
        .flatten()
        .find_map(|item| match item {
            Value::String(text) if !text.is_empty() => Some(text.clone()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_failure_message_from_verification_response() {
        let value = serde_json::json!({
            "message": "email already exists",
            "success": false
        });

        assert_eq!(extract_reason(&value).as_deref(), Some("email already exists"));
    }
}
