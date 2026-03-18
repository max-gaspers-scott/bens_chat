mod models;
use crate::models::*;

use axum::http::Method;
use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{self, Path, Query},
    routing::{get, post},
};
use minio_rsc::{Minio, client::PresignedArgs, provider::StaticProvider};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;
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
        .route("/user-chats", get(get_user_chats))
        .route("/messages", post(post_message))
        .route("/users", post(post_user))
        .route("/chats", post(post_chat))
        .route("/user-chats", post(post_user_chat))
        .route(
            "/messages",
            get(get_message_id_sender_id_content_sent_at_chat_id),
        )
        .route("/users", get(get_user_id_username))
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

#[derive(Debug, Deserialize)]
struct chat_id_query {
    chat_id: uuid::Uuid,
}

async fn get_message_id_sender_id_content_sent_at_chat_id(
    match_val: Query<chat_id_query>,
    extract::State(pool): extract::State<PgPool>,
) -> Json<Value> {
    let query = "SELECT * FROM messages WHERE chat_id = $1";
    let q = sqlx::query_as::<_, Message>(&query).bind(match_val.chat_id.clone());

    let elemint = q.fetch_all(&pool).await;

    match elemint {
        Ok(elemint) => Json(json!({
            "status": "success",
            "payload": elemint
        })),
        Err(e) => Json(json!({
            "status": "error",
            "error": e.to_string()
        })),
    }
}

// db teble names have a s at the end that is removed in struct name
// you will need to add serde Deserialize and Deserialize to the structs
pub async fn post_message(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<Message>,
) -> Json<Value> {
    // change hardcoded number of values
    let query = "INSERT INTO messages (chat_id, sender_id, content, sent_at) VALUES ($1, $2, $3, $4) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, Message>(&query)
        .bind(payload.chat_id)
        .bind(payload.sender_id)
        .bind(payload.content)
        .bind(payload.sent_at);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

// db teble names have a s at the end that is removed in struct name
// you will need to add serde Deserialize and Deserialize to the structs
pub async fn post_user(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<User>,
) -> Json<Value> {
    // change hardcoded number of values
    let query = "INSERT INTO users (username, email, password_hash, phone_number, created_at) VALUES ($1, $2, $3, $4, $5) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, User>(&query)
        .bind(payload.username)
        .bind(payload.email)
        .bind(payload.password_hash)
        .bind(payload.phone_number)
        .bind(payload.created_at);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

// db teble names have a s at the end that is removed in struct name
// you will need to add serde Deserialize and Deserialize to the structs
pub async fn post_chat(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<Chat>,
) -> Json<Value> {
    // change hardcoded number of values
    let query = "INSERT INTO chats (chat_name, created_at) VALUES ($1, $2) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, Chat>(&query)
        .bind(payload.chat_name)
        .bind(payload.created_at);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

// db teble names have a s at the end that is removed in struct name
// you will need to add serde Deserialize and Deserialize to the structs
pub async fn post_user_chat(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<UserChat>,
) -> Json<Value> {
    let query = "INSERT INTO user_chats (user_id, chat_id) VALUES ($1, $2) RETURNING *";

    let q = sqlx::query_as::<_, UserChat>(&query)
        .bind(payload.user_id)
        .bind(payload.chat_id);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

#[derive(Debug, Deserialize)]
struct GetUserChatsQuery {
    user_id: uuid::Uuid,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
struct ChatWithJoinedAt {
    pub chat_id: uuid::Uuid,
    pub chat_name: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub joined_at: Option<chrono::DateTime<chrono::Utc>>,
}

async fn get_user_chats(
    extract::Query(params): extract::Query<GetUserChatsQuery>,
    extract::State(pool): extract::State<PgPool>,
) -> Json<Value> {
    let query = r#"
        SELECT c.chat_id, c.chat_name, c.created_at, uc.joined_at
        FROM chats c
        INNER JOIN user_chats uc ON c.chat_id = uc.chat_id
        WHERE uc.user_id = $1
    "#;

    let result = sqlx::query_as::<_, ChatWithJoinedAt>(query)
        .bind(params.user_id)
        .fetch_all(&pool)
        .await;

    match result {
        Ok(chats) => Json(json!({"status": "success", "data": chats})),
        Err(e) => Json(json!({"status": "error", "error": e.to_string()})),
    }
}

#[derive(Debug, Deserialize)]
struct UsernameQuery {
    username: String,
}

async fn get_user_id_username(
    match_val: Query<UsernameQuery>,
    extract::State(pool): extract::State<PgPool>,
) -> Json<Value> {
    let query = "SELECT * FROM users WHERE username = $1";
    let q = sqlx::query_as::<_, User>(&query).bind(match_val.username.clone());

    let elemint = q.fetch_optional(&pool).await;

    match elemint {
        Ok(Some(elemint)) => Json(json!({
            "status": "success",
            "payload": {
                "user_id": elemint.user_id,
                "username": elemint.username
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
