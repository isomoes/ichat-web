use std::sync::Arc;

use axum::{Json, extract::State};
use entity::{prelude::*, user};
use sea_orm::prelude::*;
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
) -> JsonResult<LoginResp> {
    if let Some(newapi_auth) = &app.newapi_auth {
        let identity = newapi_auth
            .login(&req.username, &req.password)
            .await
            .map_err(|error| {
                Json(Error {
                    error: ErrorKind::LoginFail,
                    reason: error.to_string(),
                })
            })?;

        let user = helper::upsert_external_user(&app, identity).await?;
        let helper::Token { token, exp } = helper::new_token(&app, user.id)?;

        return Ok(Json(LoginResp { token, exp }));
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

    Ok(Json(LoginResp { token, exp }))
}
