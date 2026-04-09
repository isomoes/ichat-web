use std::sync::Arc;

use axum::{Extension, Json, extract::State};
use entity::prelude::*;
use serde::{Deserialize, Serialize};
use sea_orm::EntityTrait;
use typeshare::typeshare;

use crate::{AppState, errors::*, middlewares::auth::UserId};

#[derive(Debug, Deserialize)]
#[typeshare]
pub struct ModelIdsReq {}

#[derive(Debug, Serialize)]
#[typeshare]
pub struct ModelIdsResp {
    pub ids: Vec<String>,
}

pub async fn route(
    State(app): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
    Json(_): Json<ModelIdsReq>,
) -> JsonResult<ModelIdsResp> {
    let ids = if let Some(newapi_auth) = &app.newapi_auth {
        let user = User::find_by_id(user_id)
            .one(&app.conn)
            .await
            .kind(ErrorKind::Internal)?
            .ok_or_else(|| Error {
                error: ErrorKind::Unauthorized,
                reason: "user not found".to_owned(),
            })?;

        let api_key = user.newapi_api_key.ok_or_else(|| Error {
            error: ErrorKind::ApiFail,
            reason: "missing New API key for current user".to_owned(),
        })?;

        newapi_auth
            .fetch_model_ids(&api_key)
            .await
            .kind(ErrorKind::ApiFail)?
    } else {
        app.openrouter.get_model_ids().await
    };

    Ok(Json(ModelIdsResp { ids }))
}
