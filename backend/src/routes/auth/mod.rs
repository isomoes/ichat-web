use std::sync::Arc;

use axum::{Router, routing::post};

use crate::AppState;

mod header_auth;
mod helper;
mod login;
mod register;
mod renew;
mod verification;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login::route))
        .route("/register", post(register::route))
        .route("/verification", post(verification::route))
        .route("/renew", post(renew::route))
        .route("/header", post(header_auth::route))
}
