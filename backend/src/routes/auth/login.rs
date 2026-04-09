use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, header},
    response::IntoResponse,
};
use entity::{prelude::*, user};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use super::helper;
use crate::{AppState, errors::*};

#[derive(Debug, Deserialize)]
#[typeshare]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[typeshare]
pub struct LoginResp {
    pub token: String,
    pub exp: String,
}

pub async fn route(
    State(app): State<Arc<AppState>>,
    Json(req): Json<LoginReq>,
) -> Result<impl IntoResponse, Json<Error>> {
    if let Some(newapi_auth) = &app.newapi_auth {
        let authenticated = newapi_auth
            .login_with_session(&req.username, &req.password)
            .await
            .map_err(|error| {
                Json(Error {
                    error: ErrorKind::LoginFail,
                    reason: error.to_string(),
                })
            })?;

        let existing_key = User::find()
            .filter(
                user::Column::ExternalUserId.eq(authenticated.identity.external_user_id.clone()),
            )
            .one(&app.conn)
            .await
            .kind(ErrorKind::Internal)?
            .and_then(|user| user.newapi_api_key);

        let api_key = match existing_key {
            Some(api_key) => Some(api_key),
            None => Some(
                newapi_auth
                    .ensure_user_api_key(&authenticated)
                    .await
                    .map_err(|error| {
                        Json(Error {
                            error: ErrorKind::LoginFail,
                            reason: error.to_string(),
                        })
                    })?,
            ),
        };

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

        return Ok((headers, Json(LoginResp { token, exp })));
    }

    let model = User::find()
        .filter(user::Column::Name.eq(req.username))
        .one(&app.conn)
        .await
        .kind(ErrorKind::Internal)?
        .ok_or("")
        .kind(ErrorKind::LoginFail)?;

    if !app.hasher.verify_password(&model.password, &req.password) {
        return Err(Json(Error {
            error: ErrorKind::LoginFail,
            reason: "".to_owned(),
        }));
    }

    let helper::Token { token, exp } = helper::new_token(&app, model.id)?;

    Ok((HeaderMap::new(), Json(LoginResp { token, exp })))
}
