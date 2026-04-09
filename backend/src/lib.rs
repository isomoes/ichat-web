#![deny(unsafe_code)]

pub mod chat;
pub mod config;
pub mod errors;
pub mod middlewares;
pub mod openrouter;
pub mod routes;
pub mod utils;

pub mod serde {
    pub use stream_json::serde::*;
}

pub mod error {
    pub use crate::openrouter::Error;
}

use std::{path::PathBuf, sync::Arc};

use anyhow::Context as _;
use axum::{Router, middleware};
use chat::Context;
use config::{DB_BUSY_TIMEOUT_MS, DEFAULT_BIND_ADDR};
use dotenv::var;
use entity::{config::Column as ConfigColumn, prelude::*};
use migration::MigratorTrait;
use mimalloc::MiMalloc;
use pasetors::{keys::SymmetricKey, version4::V4};
use sea_orm::{
    ColumnTrait, ConnectionTrait, Database, DbConn, EntityTrait, PaginatorTrait, QueryFilter,
};
use tokio::{net::TcpListener, signal};
use utils::{blob::BlobDB, newapi_auth::NewApiAuthClient, password_hash::Hasher};

#[cfg(feature = "tracing")]
use tracing::info_span;

#[cfg(feature = "dev")]
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub struct AppState {
    pub conn: DbConn,
    pub key: SymmetricKey<V4>,
    pub hasher: Hasher,
    pub chat: Arc<Context>,
    pub openrouter: Arc<crate::openrouter::Openrouter>,
    pub blob: Arc<BlobDB>,
    pub auth_header: Option<String>,
    pub newapi_auth: Option<Arc<NewApiAuthClient>>,
}

fn load_api_key() -> String {
    match (var("API_KEY"), var("OPENAI_API_KEY")) {
        (Ok(key), _) => key,
        (_, Ok(key)) => key,
        _ => {
            println!("Error: API_KEY environment variable not found.");
            println!("Note: ichat read environment variable as well as .env file.");
            println!("You can get a key from https://openrouter.ai/keys");
            println!("Or use alternative setup:");
            println!(
                "- configuration: https://isomoes.github.io/ichat-web/user/config/environment"
            );
            println!("- documentation: https://isomoes.github.io/ichat-web/");

            #[cfg(windows)]
            {
                use std::io::{self, Read};
                println!("Press Enter to exit...");
                io::stdin().read_exact(&mut [0u8]).unwrap();
            }
            std::process::exit(1);
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn ensure_external_users_mapped(
    conn: &DbConn,
    external_auth: Option<&Arc<NewApiAuthClient>>,
) -> anyhow::Result<()> {
    if external_auth.is_none() {
        return Ok(());
    }

    remove_seeded_admin_for_external_auth(conn).await?;

    let unmapped_users = User::find()
        .filter(entity::user::Column::ExternalUserId.is_null())
        .count(conn)
        .await?;

    if unmapped_users > 0 {
        anyhow::bail!(
            "external auth is enabled, but {} local users are missing external_user_id mappings",
            unmapped_users
        );
    }

    Ok(())
}

async fn remove_seeded_admin_for_external_auth(conn: &DbConn) -> anyhow::Result<()> {
    let seeded_admin = User::find()
        .filter(entity::user::Column::ExternalUserId.is_null())
        .filter(entity::user::Column::Name.eq("admin"))
        .one(conn)
        .await?;

    let Some(seeded_admin) = seeded_admin else {
        return Ok(());
    };

    User::delete_by_id(seeded_admin.id).exec(conn).await?;

    Ok(())
}

pub async fn build_state_from_env() -> anyhow::Result<(Arc<AppState>, String)> {
    let api_key = load_api_key();
    let api_base = var("API_BASE").unwrap_or_else(|_| {
        var("OPENAI_API_BASE").unwrap_or("https://openrouter.ai/api".to_string())
    });
    let force_openrouter = var("FORCE_OPENROUTER_MODE")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);
    let data_path = PathBuf::from(var("DATA_PATH").unwrap_or(".".to_owned()));
    let mut database_path = data_path.clone();
    database_path.push("db.sqlite");
    let database_url = format!(
        "sqlite://{}?mode=rwc",
        database_path.display().to_string().replace('\\', "/")
    );
    let mut blob_path = data_path.clone();
    blob_path.push("blobs.redb");
    let bind_addr = var("BIND_ADDR").unwrap_or(DEFAULT_BIND_ADDR.to_owned());

    #[cfg(feature = "tracing")]
    let _db_span = info_span!("database_initialization").entered();

    migration::migrate(&database_url).await?;

    let conn = Database::connect(database_url).await?;
    migration::Migrator::up(&conn, None).await?;

    conn.execute(sea_orm::Statement::from_string(
        conn.get_database_backend(),
        &format!(
            "PRAGMA journal_mode = WAL;PRAGMA synchronous = normal;PRAGMA busy_timeout={};",
            DB_BUSY_TIMEOUT_MS
        ),
    ))
    .await?;

    let key = SymmetricKey::from(
        &Config::find()
            .filter(ConfigColumn::Key.eq("paseto_key"))
            .one(&conn)
            .await?
            .context("Cannot find paseto key")?
            .value,
    )
    .context("Cannot parse paseto key")?;

    let openrouter = Arc::new(openrouter::Openrouter::new(
        api_key,
        api_base,
        force_openrouter,
    ));

    let blob = Arc::new(BlobDB::new_from_path(blob_path).await?);
    let chat = Arc::new(Context::new(
        conn.clone(),
        openrouter.clone(),
        blob.clone(),
    )?);
    let auth_header = var("TRUSTED_HEADER").ok();
    let newapi_auth = NewApiAuthClient::from_env()?.map(Arc::new);

    ensure_external_users_mapped(&conn, newapi_auth.as_ref()).await?;

    utils::file_cleanup::FileCleanupService::new(conn.clone(), blob.clone()).start();

    Ok((
        Arc::new(AppState {
            conn,
            key,
            hasher: Hasher::default(),
            chat,
            openrouter,
            blob,
            auth_header,
            newapi_auth,
        }),
        bind_addr,
    ))
}

pub fn build_app(state: Arc<AppState>) -> Router {
    #[cfg(feature = "tracing")]
    let _router_span = info_span!("router_setup").entered();

    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .nest("/chat", routes::chat::routes())
                .nest("/user", routes::user::routes())
                .nest("/data", routes::data::routes())
                .nest("/message", routes::message::routes())
                .nest("/model", routes::model::routes())
                .layer(middlewares::compression::ZstdCompressionLayer)
                .nest("/file", routes::file::routes())
                .layer(middleware::from_extractor_with_state::<
                    middlewares::auth::Middleware,
                    _,
                >(state.clone()))
                .nest("/auth", routes::auth::routes())
                .layer(middlewares::logger::LoggerLayer),
        )
        .fallback(routes::spa::spa_handler)
        .with_state(state);

    #[cfg(feature = "dev")]
    let app = app.layer(
        CorsLayer::new()
            .allow_methods(AllowMethods::any())
            .allow_origin(AllowOrigin::any())
            .allow_headers(AllowHeaders::list([
                http::header::AUTHORIZATION,
                http::header::CONTENT_TYPE,
            ])),
    );

    app
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn external_auth_allows_seeded_admin_cleanup() {
        let conn = Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite should connect");
        migration::Migrator::up(&conn, None)
            .await
            .expect("migrations should apply");

        let external_auth = Arc::new(NewApiAuthClient {
            base_url: "http://localhost".to_owned(),
            user_header: "New-Api-User".to_owned(),
            service_bearer: None,
            http: reqwest::Client::new(),
        });

        ensure_external_users_mapped(&conn, Some(&external_auth))
            .await
            .expect("seeded admin should not block external auth startup");

        let user_count = User::find()
            .count(&conn)
            .await
            .expect("user count should load");
        assert_eq!(user_count, 0);
    }
}

pub async fn run() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    crate::utils::logger::init();

    #[cfg(feature = "tracing")]
    let _main_span = info_span!("ichat_backend_startup").entered();

    let (state, bind_addr) = build_state_from_env().await?;
    let app = build_app(state);

    log::info!("Listening on http://{}", bind_addr);

    #[cfg(feature = "tracing")]
    let _server_span = info_span!("server_startup", bind_addr = %bind_addr).entered();

    let tcp = TcpListener::bind(&bind_addr).await?;
    axum::serve(tcp, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
