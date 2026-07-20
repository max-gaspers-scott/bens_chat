use crate::models::User;
mod models;
use crate::models::User;
mod models;
use crate::models::User;
mod models;
use crate::models::*;
mod models;
use axum::http::Method;
use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{self, Multipart, Path, Query},
    routing::{get, post},
};
use minio_rsc::{Minio, client::PresignedArgs, provider::StaticProvider};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;
use sqlx::Pool;
use sqlx::types::chrono::Utc;
use sqlx::{postgres::PgPoolOptions, prelude::FromRow};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::result::Result;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

use axum::response::{Html, IntoResponse};
use tower::service_fn;
use tower_http::services::ServeDir;
async fn health() -> String {
    "healthy".to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://dbuser:p@localhost:1111/data".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(100)
        .connect(&db_url)
        .await?;

    let migrate = sqlx::migrate!("./migrations").run(&pool).await;

    match migrate {
        Ok(_) => println!("Migrations applied successfully."),
        Err(e) => eprintln!("Error applying migrations: {}", e),
    };

    let static_service =
        ServeDir::new("frontend/build").not_found_service(service_fn(|_req| async {
            match tokio::fs::read_to_string("frontend/build/index.html").await {
                Ok(body) => Ok((StatusCode::OK, Html(body)).into_response()),
                Err(err) => Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read index.html: {}", err),
                )
                    .into_response()),
            }
        }));

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/post_user", post(post_user))
        .route("/api/post_message", post(post_message))
        .route(
            "/api/get_username_email_user_id",
            get(get_username_email_user_id),
        )
        .route(
            "/api/post_conversation_participant",
            post(post_conversation_participant),
        )
        .route("/api/post_conversation", post(post_conversation))
        .fallback_service(static_service)
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(vec![
                    "http://localhost:3000".parse().unwrap(),
                    "https://example.com".parse().unwrap(),
                ]))
                .allow_methods([Method::GET, Method::POST])
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await.unwrap();

    axum::serve(listener, app).await.unwrap();
    Ok(())
}
// you will need to add serde Deserialize and Deserialize to the structs
pub async fn post_conversation(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<Conversation>,
) -> Json<Value> {
    // change hardcoded number of values
    let query = "INSERT INTO conversations (name, is_group_chat) VALUES ($1, $2) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, Conversation>(&query)
        .bind(payload.name)
        .bind(payload.is_group_chat);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

// db teble names have a s at the end that is removed in struct name
// you will need to add serde Deserialize and Deserialize to the structs
pub async fn post_conversation_participant(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<ConversationParticipant>,
) -> Json<Value> {
    // change hardcoded number of values
    let query = "INSERT INTO conversation_participants (conversation_id, user_id) VALUES ($1, $2) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, ConversationParticipant>(&query)
        .bind(payload.conversation_id)
        .bind(payload.user_id);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

// db teble names have a s at the end that is removed in struct name
// you will need to add serde Deserialize and Deserialize to the structs
pub async fn post_message(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<Message>,
) -> Json<Value> {
    // change hardcoded number of values
    let query = "INSERT INTO messages (conversation_id, sender_id, content) VALUES ($1, $2, $3) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, Message>(&query)
        .bind(payload.conversation_id)
        .bind(payload.sender_id)
        .bind(payload.content);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

#[derive(Debug, Deserialize)]
struct user_id_query {
    user_id: uuid::Uuid,
}

async fn get_username_email_user_id(
    match_val: Query<user_id_query>,
    extract::State(pool): extract::State<PgPool>,
) -> Json<Value> {
    let query = format!("SELECT * FROM users WHERE user_id = $1");
    let q = sqlx::query_as::<_, User>(&query).bind(match_val.user_id.clone());

    let elemint = q.fetch_optional(&pool).await;

    match elemint {
        Ok(Some(elemint)) => Json(json!({
            "status": "success",
            "payload": {
            "username": elemint.username,
        "email": elemint.email,

            }
        })),
        Ok(None) => Json(json!({
            "status": "error",
            "error": "User not found"
        })),
        _ => Json(json!({
            "status": "error",
            "error": "User not found"
        })),
    }
}
// db teble names have a s at the end that is removed in struct name
// you will need to add serde Deserialize and Deserialize to the structs
pub async fn post_user(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<User>,
) -> Json<Value> {
    // change hardcoded number of values
    let query = "INSERT INTO users (username, email, display_name) VALUES ($1, $2, $3) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, User>(&query)
        .bind(payload.username)
        .bind(payload.email)
        .bind(payload.display_name);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}
