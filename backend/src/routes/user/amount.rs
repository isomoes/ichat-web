use std::sync::Arc;

use axum::{Extension, Json, extract::State, http::HeaderMap};
use entity::prelude::User;
use sea_orm::EntityTrait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    AppState,
    errors::{Error, ErrorKind, WithKind},
    middlewares::auth::UserId,
};

#[derive(Debug, Deserialize)]
pub struct AmountReq {
    pub amount: i64,
}

pub async fn route(
    State(app): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    headers: HeaderMap,
    Json(req): Json<AmountReq>,
) -> Result<Json<Value>, Json<Error>> {
    let newapi_auth = app.newapi_auth.as_ref().ok_or_else(|| {
        Json(Error {
            error: ErrorKind::MalformedRequest,
            reason: "External auth is not enabled".to_owned(),
        })
    })?;

    let user = User::find_by_id(user_id)
        .one(&app.conn)
        .await
        .kind(ErrorKind::Internal)?
        .ok_or_else(|| {
            Json(Error {
                error: ErrorKind::Unauthorized,
                reason: "user not found".to_owned(),
            })
        })?;

    let session_cookie = crate::utils::newapi_auth::extract_browser_session_cookie(&headers)
        .ok_or_else(|| {
            Json(Error {
                error: ErrorKind::Unauthorized,
                reason: "missing New API session cookie for current user".to_owned(),
            })
        })?;
    let external_user_id = user.external_user_id.ok_or_else(|| {
        Json(Error {
            error: ErrorKind::Unauthorized,
            reason: "missing New API external user id for current user".to_owned(),
        })
    })?;

    let value = newapi_auth
        .post_with_session_cookie(
            "/api/user/amount",
            &external_user_id,
            &session_cookie,
            json!({ "amount": req.amount }),
        )
        .await
        .kind(ErrorKind::ApiFail)?;

    Ok(Json(value))
}
