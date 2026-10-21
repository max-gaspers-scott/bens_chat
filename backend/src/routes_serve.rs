use axum::{
    Extension, Json, Router,
    extract::{self, Query},
    middleware,
    routing::{get, post},
};
use bcrypt::{DEFAULT_COST, hash, verify};
use bens_chat_shared::{Connect4, Position};
use minio_rsc::Minio;
use minio_rsc::client::PresignedArgs;
use minio_rsc::provider::StaticProvider;
use serde::Deserialize;
use serde_json::{Value, json};
use socketioxide::SocketIo;
use sqlx::PgPool;
use std::{env, result::Result};
use uuid::Uuid;

use crate::{auth, auth::AuthUser, data, models::*};

use crate::axum_handelers;
use crate::routes_serve;

pub fn routes() -> Router<PgPool> {
    let public_routes = Router::new()
        .route("/health", get(axum_handelers::health))
        .route("/users", post(axum_handelers::post_user))
        .route("/test", get(|| async { "hi fe" }))
        .route("/auth/login", post(axum_handelers::login_user));

    // .route("/debug-headers", get(debug_headers));

    let protected_routes = Router::new()
        .route(
            "/user-chats",
            get(axum_handelers::get_users_chats).post(axum_handelers::post_chat_participant), // need
                                                                                              // a real get part so users can see there "chats"
        )
        .route(
            "/messages",
            get(axum_handelers::get_message_id_sender_name_content_parent)
                .post(axum_handelers::post_message)
                // .patch(update_message)
                .patch(axum_handelers::update_connect),
        )
        .route("/password-set", post(axum_handelers::set_password))
        .route("/minio-fetch", get(axum_handelers::get_fetch_url))
        .route("/minio-post", get(axum_handelers::get_put_url))
        .layer(middleware::from_fn(auth::authorize));

    Router::new().merge(public_routes).merge(protected_routes)
}
