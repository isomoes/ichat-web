use std::sync::Arc;

use axum::{Router, routing::post};

use crate::AppState;

mod amount;
mod create;
mod delete;
mod list;
mod pay;
mod read;
mod update;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/amount", post(amount::route))
        .route("/create", post(create::route))
        .route("/delete", post(delete::route))
        .route("/read", post(read::route))
        .route("/update", post(update::route))
        .route("/pay", post(pay::route))
        .route("/list", post(list::route))
}
