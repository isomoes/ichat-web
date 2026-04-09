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
    api_key: Option<String>,
) -> Result<entity::user::Model, AppError> {
    let existing = User::find()
        .filter(user::Column::ExternalUserId.eq(identity.external_user_id.clone()))
        .one(&app.conn)
        .await
        .kind(ErrorKind::Internal)?;

    match existing {
        Some(existing) => {
            if existing.name == identity.username && api_key == existing.newapi_api_key {
                return Ok(existing);
            }

            let mut active_model = existing.into_active_model();
            active_model.name = Set(identity.username);
            active_model.newapi_api_key = Set(api_key);
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
                newapi_api_key: Set(api_key),
                ..Default::default()
            };

            user.insert(&app.conn).await.kind(ErrorKind::Internal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chat, utils::blob::BlobDB};
    use entity::prelude::Config;
    use migration::MigratorTrait;
    use pasetors::version4::V4;
    use sea_orm::{Database, EntityTrait};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn upsert_external_user_stores_and_updates_newapi_api_key() {
        let conn = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite should connect");
        migration::Migrator::up(&conn, None)
            .await
            .expect("migrations should apply");

        let config = Config::find()
            .one(&conn)
            .await
            .expect("config query should succeed")
            .expect("paseto config should exist");
        let key = pasetors::keys::SymmetricKey::<V4>::from(&config.value)
            .expect("paseto key should parse");

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic enough")
            .as_nanos();
        let blob_path = std::env::temp_dir().join(format!("ichat-helper-{unique}.redb"));
        let blob = BlobDB::new_from_path(blob_path)
            .await
            .expect("blob db should initialize");
        let openrouter = Arc::new(crate::openrouter::Openrouter::new(
            "global-key",
            "https://openrouter.ai/api",
            false,
        ));
        let blob = Arc::new(blob);
        let chat =
            Arc::new(chat::Context::new(conn.clone(), openrouter.clone(), blob.clone()).unwrap());

        let app = AppState {
            conn: conn.clone(),
            key,
            hasher: crate::utils::password_hash::Hasher::default(),
            chat,
            openrouter,
            blob,
            auth_header: None,
            newapi_auth: None,
        };

        let created = upsert_external_user(
            &app,
            ExternalIdentity {
                external_user_id: "42".to_owned(),
                username: "alice".to_owned(),
            },
            Some("sk-first".to_owned()),
        )
        .await
        .expect("initial upsert should succeed");

        assert_eq!(created.name, "alice");
        assert_eq!(created.newapi_api_key.as_deref(), Some("sk-first"));

        let updated = upsert_external_user(
            &app,
            ExternalIdentity {
                external_user_id: "42".to_owned(),
                username: "alice-renamed".to_owned(),
            },
            Some("sk-second".to_owned()),
        )
        .await
        .expect("update upsert should succeed");

        assert_eq!(updated.name, "alice-renamed");
        assert_eq!(updated.newapi_api_key.as_deref(), Some("sk-second"));
    }
}
