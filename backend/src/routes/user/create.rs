use std::sync::Arc;

use axum::{Extension, Json, extract::State};
use entity::{prelude::*, user};
use sea_orm::{ActiveValue, EntityTrait};
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

use crate::{AppState, errors::*, middlewares::auth::UserId};

#[derive(Debug, Deserialize)]
#[typeshare]
pub struct UserCreateReq {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[typeshare]
pub struct UserCreateResp {
    pub user_id: i32,
}

pub async fn route(
    State(app): State<Arc<AppState>>,
    Extension(UserId(_)): Extension<UserId>,
    Json(req): Json<UserCreateReq>,
) -> JsonResult<UserCreateResp> {
    if app.newapi_auth.is_some() {
        return Err(Json(Error {
            error: ErrorKind::MalformedRequest,
            reason: "User creation is managed by the external auth provider".to_owned(),
        }));
    }

    let password_hash = app.hasher.hash_password(&req.password);
    let new_user = user::ActiveModel {
        name: ActiveValue::Set(req.username),
        password: ActiveValue::Set(password_hash),
        ..Default::default()
    };

    let new_user = User::insert(new_user)
        .exec(&app.conn)
        .await
        .kind(ErrorKind::Internal)?;

    Ok(Json(UserCreateResp {
        user_id: new_user.last_insert_id,
    }))
}
