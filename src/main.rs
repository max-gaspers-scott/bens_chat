mod models;
use crate::models::Message;
use crate::models::User;

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

async fn generate_signed_url(object_key: String) -> Result<String, anyhow::Error> {
    let endpoint = env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "localhost:9001".to_string());
    let access_key = env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string());
    let secret_key = env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| "minioadmin".to_string());
    let bucket = env::var("MINIO_BUCKET").unwrap_or_else(|_| "bucket".to_string());
    let endpoint = env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "localhost:9000".to_string());
    let secure = env::var("MINIO_SECURE")
        .map(|s| s.to_lowercase() == "true")
        .unwrap_or(false);

    let provider = StaticProvider::new(&access_key, &secret_key, None);

    let minio = Minio::builder()
        .endpoint(&endpoint)
        .provider(provider)
        .secure(secure)
        .region("us-east-1".to_string()) // Explicitly set region to match MinIO default
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create MinIO client: {}", e))?;

    let presigned_url = minio
        .presigned_get_object(
            PresignedArgs::new(bucket, object_key).expires(3600), // 1 hour in seconds
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to generate presigned URL: {}", e))?;
    Ok(presigned_url)
}

async fn get_signed_url(Path(video_path): Path<String>) -> impl IntoResponse {
    let object_key = video_path;
    println!("Environment variables:");
    println!(
        "MINIO_ENDPOINT: {}",
        env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "not set".to_string())
    );
    println!(
        "MINIO_BUCKET: {}",
        env::var("MINIO_BUCKET").unwrap_or_else(|_| "not set, using default 'test'".to_string())
    );

    match generate_signed_url(object_key).await {
        Ok(url) => (StatusCode::OK, url).into_response(),
        Err(e) => {
            eprintln!("Error generating signed URL: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to generate signed URL: {}", e),
            )
                .into_response()
        }
    }
}
async fn upload_video(mut multipart: Multipart) -> Result<Json<Value>, (StatusCode, String)> {
    let endpoint = env::var("MINIO_ENDPOINT").unwrap_or_else(|_| "minio:9000".to_string());
    let access_key = env::var("MINIO_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string());
    let secret_key = env::var("MINIO_SECRET_KEY").unwrap_or_else(|_| "minioadmin".to_string());
    let bucket = env::var("MINIO_BUCKET").unwrap_or_else(|_| "bucket".to_string());
    let secure = env::var("MINIO_SECURE")
        .map(|s| s.to_lowercase() == "true")
        .unwrap_or(false);

    let provider = StaticProvider::new(&access_key, &secret_key, None);
    let minio = Minio::builder()
        .endpoint(&endpoint)
        .provider(provider)
        .secure(secure)
        .build()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create MinIO client: {}", e),
            )
        })?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {}", e)))?
    {
        // Accept any field that carries a filename (the uploaded file)
        let original_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload.mp4".to_string());

        // Prefix with a UUID so every upload gets a unique key
        let object_key = format!("{}/{}", uuid::Uuid::new_v4(), original_name);

        let data = field.bytes().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read file bytes: {}", e),
            )
        })?;

        minio
            .put_object(&bucket, &object_key, data)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("MinIO upload failed: {}", e),
                )
            })?;

        // Build the public URL using the browser-accessible MinIO endpoint
        let public_endpoint = env::var("MINIO_PUBLIC_ENDPOINT")
            .unwrap_or_else(|_| "localhost:9000".to_string());
        let scheme = if secure { "https" } else { "http" };
        let url = format!("{}://{}/{}/{}", scheme, public_endpoint, bucket, object_key);

        return Ok(Json(json!({
            "status": true,
            "message": "File uploaded successfully",
            "object_key": object_key,
            "url": url,
        })));
    }

    Err((
        StatusCode::BAD_REQUEST,
        "No file field found in the request".to_string(),
    ))
}

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
        .route("/signed-urls/:video_path", get(get_signed_url))
        .route("/upload", post(upload_video))
        .route("/python", get(python))
        .route("/users", post(post_user))
        .route("/messages", post(post_message))
        .route("/get-messages", get(get_content_sent_at_sender_id))
        .route("/get_user_id", get(get_user_id_username))
        .route("/all-users", get(get_id_username_email))
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

#[derive(sqlx::FromRow, Debug, Deserialize)]
struct SenderIdQuery {
    sender_id: uuid::Uuid,
    receiver_id: uuid::Uuid,
}

// need to add ..
// mod models;
// use crate::models::User;

async fn get_content_sent_at_sender_id(
    extract::State(pool): extract::State<PgPool>,
    match_val: Query<SenderIdQuery>,
) -> Result<Json<Vec<Message>>, (StatusCode, String)> {
    let query = "SELECT * FROM messages WHERE sender_id = $1 AND receiver_id = $2 ORDER BY sent_at";

    let q = sqlx::query_as::<_, Message>(&query)
        .bind(match_val.sender_id)
        .bind(match_val.receiver_id);

    let elemint = q.fetch_all(&pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database err{}", e),
        )
    })?;

    Ok(Json(elemint))
}
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub username: String,
    pub password_hash: String,
}
// db teble names have a s at the end that is removed in struct name
// you will need to add serde Deserialize and Deserialize to the structs
pub async fn post_user(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<CreateUser>,
) -> Json<Value> {
    // change hardcoded number of values
    let query =
        "INSERT INTO users (email, username, password_hash) VALUES ($1, $2, $3) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, CreateUser>(&query)
        .bind(payload.email)
        .bind(payload.username)
        .bind(payload.password_hash);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

// db teble names have a s at the end that is removed in struct name
// you will need to add serde Deserialize and Deserialize to the structs
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct CreateMessage {
    pub sender_id: uuid::Uuid,
    pub receiver_id: uuid::Uuid,
    pub content: String,
}
pub async fn post_message(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<CreateMessage>,
) -> Json<Value> {
    // change hardcoded number of values
    let query =
        "INSERT INTO messages (sender_id, receiver_id, content) VALUES ($1, $2, $3) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, CreateMessage>(&query)
        .bind(payload.sender_id)
        .bind(payload.receiver_id)
        .bind(payload.content);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

async fn python() -> Result<Json<Value>, (StatusCode, String)> {
    // Call the Python FastAPI service
    let client = reqwest::Client::new();
    let res = client
        .get("http://python:8003/chat") // Use service name and correct port
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Request failed: {}", e),
            )
        })?;

    if res.status().is_client_error() || res.status().is_server_error() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Error from Python service: {}", res.status()),
        ));
    }

    let json_response: Value = res.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse JSON: {}", e),
        )
    })?;

    Ok(Json(json!({"payload": json_response})))
}

#[derive(sqlx::FromRow, Debug, Deserialize)]
struct UsernameQuery {
    username: String,
}

// need to add ..
// mod models;
// use crate::models::User;

async fn get_user_id_username(
    extract::State(pool): extract::State<PgPool>,
    match_val: Query<UsernameQuery>,
) -> Result<Json<Vec<User>>, (StatusCode, String)> {
    let query = "SELECT * FROM users WHERE username = $1";

    let q = sqlx::query_as::<_, User>(&query).bind(match_val.username.clone());

    let elemint = q.fetch_all(&pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database err{}", e),
        )
    })?;

    Ok(Json(elemint))
}

async fn get_id_username_email(
    extract::State(pool): extract::State<PgPool>,
) -> Result<Json<Vec<User>>, (StatusCode, String)> {
    let query = "SELECT id, username, email, password_hash, created_at FROM users";

    let q = sqlx::query_as::<_, User>(&query);

    let elemint = q.fetch_all(&pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database err{}", e),
        )
    })?;

    Ok(Json(elemint))
}
