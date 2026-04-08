use crate::{
    config::TOKEN_EXPIRATION_SECS,
    AppState,
    errors::{AppError, ErrorKind, WithKind},
    utils::newapi_auth::ExternalIdentity,
};
use entity::{prelude::User, user};
use pasetors::{claims::Claims, local};
use protocol::UserPreference;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
};
use std::time::Duration;

pub struct Token {
    pub token: String,
    pub exp: String,
}
pub fn new_token(app: &AppState, user_id: i32) -> Result<Token, AppError> {
    let mut claim = Claims::new().kind(ErrorKind::Internal)?;

    let expiration = Duration::from_secs(TOKEN_EXPIRATION_SECS);
    claim
        .set_expires_in(&expiration)
        .kind(ErrorKind::Internal)?;

    // safety:
    // "uid" is not reserve
    claim.add_additional("uid", user_id).unwrap();

    // safety:
    // "exp" must exists
    let exp = claim.get_claim("exp").unwrap().as_str().unwrap().to_owned();

    let token = local::encrypt(&app.key, &claim, None, None).kind(ErrorKind::Internal)?;

    Ok(Token { token, exp })
}

pub async fn upsert_external_user(
    app: &AppState,
    identity: ExternalIdentity,
) -> Result<entity::user::Model, AppError> {
    let existing = User::find()
        .filter(user::Column::ExternalUserId.eq(identity.external_user_id.clone()))
        .one(&app.conn)
        .await
        .kind(ErrorKind::Internal)?;

    match existing {
        Some(existing) => {
            if existing.name == identity.username {
                return Ok(existing);
            }

            let mut active_model = existing.into_active_model();
            active_model.name = Set(identity.username);
            active_model
                .update(&app.conn)
                .await
                .kind(ErrorKind::Internal)
        }
        None => {
            let user = user::ActiveModel {
                name: Set(identity.username),
                password: Set(String::new()),
                preference: Set(UserPreference {
                    theme: None,
                    locale: None,
                    submit_on_enter: None,
                }),
                external_user_id: Set(Some(identity.external_user_id)),
                ..Default::default()
            };

            user.insert(&app.conn).await.kind(ErrorKind::Internal)
        }
    }
}
