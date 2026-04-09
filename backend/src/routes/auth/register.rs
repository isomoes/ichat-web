use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use super::helper;
use crate::{AppState, errors::*};

#[derive(Debug, Deserialize)]
#[typeshare]
pub struct RegisterReq {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub verification_code: Option<String>,
    pub aff_code: Option<String>,
}

#[derive(Debug, Serialize)]
#[typeshare]
pub struct RegisterResp {
    pub token: String,
    pub exp: String,
}

pub async fn route(
    State(app): State<Arc<AppState>>,
    Json(req): Json<RegisterReq>,
) -> Result<impl IntoResponse, Json<Error>> {
    let Some(newapi_auth) = &app.newapi_auth else {
        return Err(Json(Error {
            error: ErrorKind::MalformedRequest,
            reason: "External registration is not enabled".to_owned(),
        }));
    };

    let authenticated = newapi_auth
        .register_with_session(
            &req.username,
            &req.password,
            req.email.as_deref(),
            req.verification_code.as_deref(),
            req.aff_code.as_deref(),
        )
        .await
        .map_err(|error| {
            Json(Error {
                error: ErrorKind::MalformedRequest,
                reason: error.to_string(),
            })
        })?;

    let api_key = Some(
        newapi_auth
            .ensure_user_api_key(&authenticated)
            .await
            .map_err(|error| {
                Json(Error {
                    error: ErrorKind::MalformedRequest,
                    reason: error.to_string(),
                })
            })?,
    );

    let session_cookie = authenticated.session_cookie.clone();
    let user = helper::upsert_external_user(&app, authenticated.identity, api_key).await?;
    let helper::Token { token, exp } = helper::new_token(&app, user.id)?;

    let mut headers = HeaderMap::new();
    if let Some(session_cookie) = session_cookie {
        headers.insert(
            header::SET_COOKIE,
            crate::utils::newapi_auth::build_browser_session_set_cookie(&session_cookie)
                .kind(ErrorKind::Internal)?,
        );
    }

    Ok((headers, Json(RegisterResp { token, exp })))
}
