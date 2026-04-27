// minio stuff
use minio_rsc::Minio;
use minio_rsc::client::PresignedArgs;
use minio_rsc::provider::StaticProvider;

mod auth;
mod models;

use minio_rsc;

use crate::{auth::AuthUser, models::*};
use axum::{
    Extension, Json, Router,
    extract::{self, Query, Request},
    http::{
        HeaderValue, Method, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    middleware,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use bcrypt::{DEFAULT_COST, hash, verify};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{env, result::Result};
use tower::service_fn;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    services::ServeDir,
};
use uuid::Uuid;

async fn health() -> String {
    "healthy".to_string()
}

async fn debug_headers(request: Request) -> Json<Value> {
    let mut headers_json = serde_json::Map::new();
    for (name, value) in request.headers() {
        if let Ok(val) = value.to_str() {
            headers_json.insert(name.to_string().to_lowercase(), json!(val));
        }
    }
    Json(json!({
        "headers": headers_json,
        "uri": request.uri().to_string(),
        "method": request.method().to_string(),
    }))
}

fn build_cors_layer() -> CorsLayer {
    let configured_origins = env::var("CORS_ALLOWED_ORIGINS").ok();

    let base = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any)
        .expose_headers([AUTHORIZATION, CONTENT_TYPE].map(|h| h.into()));

    match configured_origins.as_deref().map(str::trim) {
        Some("*") => base.allow_origin(Any),
        Some(origins) if !origins.is_empty() => {
            let parsed_origins = origins
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(|origin| origin.parse::<HeaderValue>().unwrap())
                .collect::<Vec<_>>();

            if parsed_origins.is_empty() {
                base.allow_origin(AllowOrigin::list(vec![
                    "http://localhost:3000".parse().unwrap(),
                    "http://127.0.0.1:3000".parse().unwrap(),
                ]))
            } else {
                base.allow_origin(AllowOrigin::list(parsed_origins))
            }
        }
        _ => base.allow_origin(AllowOrigin::list(vec![
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
        ])),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

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
        ServeDir::new("../frontend/build").not_found_service(service_fn(|_req| async {
            match tokio::fs::read_to_string("../frontend/build/index.html").await {
                Ok(body) => Ok((StatusCode::OK, Html(body)).into_response()),
                Err(err) => Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read index.html: {}", err),
                )
                    .into_response()),
            }
        }));

    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/users", post(post_user))
        .route("/auth/login", post(login_user))
        .route("/debug-headers", get(debug_headers));

    let protected_routes = Router::new()
        .route("/user-chats", get(get_user_chats).post(post_user_chat))
        .route(
            "/messages",
            get(get_message_id_sender_id_content_sent_at_chat_id).post(post_message),
        )
        .route("/users", get(get_user_id_username))
        .route("/chats", post(post_chat))
        .route("/minio-fetch", get(get_fetch_url))
        .route("/minio-post", get(get_put_url))
        .layer(middleware::from_fn(auth::authorize));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .fallback_service(static_service)
        .layer(build_cors_layer())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

#[derive(Debug, Deserialize)]
struct ChatIdQuery {
    chat_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct CreateMessageRequest {
    chat_id: Uuid,
    content: String,
    minio_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    username: String,
    email: String,
    password: String,
    phone_number: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct CreateChatRequest {
    chat_name: Option<String>,
}

async fn is_user_in_chat(pool: &PgPool, user_id: Uuid, chat_id: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_chats WHERE user_id = $1 AND chat_id = $2)",
    )
    .bind(user_id)
    .bind(chat_id)
    .fetch_one(pool)
    .await
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct MessageWithUser {
    pub message_id: uuid::Uuid,
    pub sender_id: uuid::Uuid,
    pub username: String,
    pub content: String,
    pub sent_at: Option<chrono::DateTime<chrono::Utc>>,
    pub minio_url: Option<String>,
}

async fn get_message_id_sender_id_content_sent_at_chat_id(
    Extension(auth_user): Extension<AuthUser>,
    match_val: Query<ChatIdQuery>,
    extract::State(pool): extract::State<PgPool>,
) -> Json<Value> {
    match is_user_in_chat(&pool, auth_user.user_id, match_val.chat_id).await {
        Ok(true) => {}
        Ok(false) => return Json(json!({"status": "error", "error": "Forbidden"})),
        Err(e) => return Json(json!({"status": "error", "error": e.to_string()})),
    }

    let q = "SELECT messages.message_id, messages.sender_id, users.username, messages.content, messages.sent_at, messages.minio_url FROM messages LEFT JOIN users ON messages.sender_id = users.user_id WHERE chat_id = $1 ORDER BY sent_at";
    let result = sqlx::query_as::<_, MessageWithUser>(q)
        .bind(match_val.chat_id)
        .fetch_all(&pool)
        .await;

    match result {
        Ok(messages) => Json(json!({"status": "success", "payload": messages})),
        Err(e) => Json(json!({"status": "error", "error": e.to_string()})),
    }
}

async fn post_message(
    Extension(auth_user): Extension<AuthUser>,
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<CreateMessageRequest>,
) -> Json<Value> {
    match is_user_in_chat(&pool, auth_user.user_id, payload.chat_id).await {
        Ok(true) => {}
        Ok(false) => return Json(json!({"res": "error: forbidden"})),
        Err(e) => return Json(json!({"res": format!("error: {}", e)})),
    }

    let result = sqlx::query_as::<_, Message>(
        "INSERT INTO messages (chat_id, sender_id, content, minio_url) VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(payload.chat_id)
    .bind(auth_user.user_id)
    .bind(payload.content)
    .bind(payload.minio_url)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

async fn post_user(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<CreateUserRequest>,
) -> Json<Value> {
    let password_hash = match hash(&payload.password, DEFAULT_COST) {
        Ok(password_hash) => password_hash,
        Err(e) => return Json(json!({"res": format!("error: {}", e)})),
    };

    let result = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, email, password_hash, phone_number) VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(payload.username)
    .bind(payload.email)
    .bind(password_hash)
    .bind(payload.phone_number)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(value) => Json(json!({
            "res": "success",
            "data": {
                "user_id": value.user_id,
                "username": value.username,
                "email": value.email,
                "phone_number": value.phone_number,
                "created_at": value.created_at,
            }
        })),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

async fn login_user(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Json<Value> {
    let result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(payload.username)
        .fetch_optional(&pool)
        .await;

    match result {
        Ok(Some(user)) if verify(&payload.password, &user.password_hash).unwrap_or(false) => {
            match auth::create_token(user.user_id, &user.username) {
                Ok(token) => Json(json!({
                    "status": "success",
                    "payload": {
                        "user_id": user.user_id,
                        "token": token,
                    }
                })),
                Err(e) => Json(json!({"status": "error", "error": e.to_string()})),
            }
        }
        Ok(_) => Json(json!({
            "status": "error",
            "error": "Invalid username or password"
        })),
        Err(e) => Json(json!({"status": "error", "error": e.to_string()})),
    }
}

async fn post_chat(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<CreateChatRequest>,
) -> Json<Value> {
    let result = sqlx::query_as::<_, Chat>("INSERT INTO chats (chat_name) VALUES ($1) RETURNING *")
        .bind(payload.chat_name)
        .fetch_one(&pool)
        .await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

async fn post_user_chat(
    Extension(auth_user): Extension<AuthUser>,
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<UserChat>,
) -> Json<Value> {
    if payload.user_id != auth_user.user_id {
        match is_user_in_chat(&pool, auth_user.user_id, payload.chat_id).await {
            Ok(true) => {}
            Ok(false) => return Json(json!({"res": "error: forbidden"})),
            Err(e) => return Json(json!({"res": format!("error: {}", e)})),
        }
    }

    let result = sqlx::query_as::<_, UserChat>(
        "INSERT INTO user_chats (user_id, chat_id) VALUES ($1, $2) RETURNING *",
    )
    .bind(payload.user_id)
    .bind(payload.chat_id)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

#[derive(Debug, Deserialize)]
struct GetUserChatsQuery {
    user_id: Uuid,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
struct ChatWithJoinedAt {
    pub chat_id: Uuid,
    pub chat_name: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub joined_at: Option<chrono::DateTime<chrono::Utc>>,
}

async fn get_user_chats(
    Extension(auth_user): Extension<AuthUser>,
    extract::Query(params): extract::Query<GetUserChatsQuery>,
    extract::State(pool): extract::State<PgPool>,
) -> Json<Value> {
    if params.user_id != auth_user.user_id {
        return Json(json!({"status": "error", "error": "Forbidden"}));
    }

    let query = r#"
        SELECT c.chat_id, c.chat_name, c.created_at, uc.joined_at
        FROM chats c
        INNER JOIN user_chats uc ON c.chat_id = uc.chat_id
        WHERE uc.user_id = $1
    "#;

    let result = sqlx::query_as::<_, ChatWithJoinedAt>(query)
        .bind(auth_user.user_id)
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
    let result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(match_val.username.clone())
        .fetch_optional(&pool)
        .await;

    match result {
        Ok(Some(user)) => Json(json!({
            "status": "success",
            "payload": {
                "user_id": user.user_id,
                "username": user.username,
            }
        })),
        Ok(None) => Json(json!({"status": "error", "error": "User not found"})),
        Err(e) => Json(json!({"status": "error", "error": e.to_string()})),
    }
}

#[derive(Debug, Deserialize)]
struct UploadUrlQuery {
    chat_id: Uuid,
    file_extension: String,
}

#[derive(Debug, Deserialize)]
struct FetchUrlQuery {
    object_key: String,
}

fn build_minio_client(endpoint: &str) -> Minio {
    let access_key = env::var("MINIO_ACCESS_KEY").expect("MINIO_ACCESS_KEY not set");
    let secret_key = env::var("MINIO_SECRET_KEY").expect("MINIO_SECRET_KEY not set");
    let secure = env::var("MINIO_SECURE")
        .map(|v| v == "true")
        .unwrap_or(false);
    let provider = StaticProvider::new(&access_key, &secret_key, None);
    Minio::builder()
        .endpoint(endpoint)
        .provider(provider)
        .secure(secure)
        .build()
        .unwrap()
}

async fn get_put_url(
    Extension(auth_user): Extension<AuthUser>,
    extract::State(pool): extract::State<PgPool>,
    Query(params): Query<UploadUrlQuery>,
) -> Json<Value> {
    match is_user_in_chat(&pool, auth_user.user_id, params.chat_id).await {
        Ok(true) => {}
        Ok(false) => return Json(json!({"status": "error", "error": "Forbidden"})),
        Err(e) => return Json(json!({"status": "error", "error": e.to_string()})),
    }

    let object_key = format!(
        "media/{}/{}.{}",
        params.chat_id,
        Uuid::new_v4(),
        params.file_extension
    );
    let public_endpoint = env::var("MINIO_PUBLIC_ENDPOINT").expect("MINIO_PUBLIC_ENDPOINT not set");
    let minio = build_minio_client(&public_endpoint);
    match minio
        .presigned_put_object(PresignedArgs::new("bucket", &object_key).expires(15 * 60))
        .await
    {
        Ok(url) => Json(json!({"status": "success", "upload_url": url, "object_key": object_key})),
        Err(e) => Json(json!({"status": "error", "error": e.to_string()})),
    }
}

async fn get_fetch_url(
    Extension(auth_user): Extension<AuthUser>,
    extract::State(pool): extract::State<PgPool>,
    Query(params): Query<FetchUrlQuery>,
) -> Json<Value> {
    // Extract chat_id from the object key (format: media/{chat_id}/{uuid}.{ext})
    let chat_id = match params
        .object_key
        .split('/')
        .nth(1)
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return Json(json!({"status": "error", "error": "Invalid object key"})),
    };
    match is_user_in_chat(&pool, auth_user.user_id, chat_id).await {
        Ok(true) => {}
        Ok(false) => return Json(json!({"status": "error", "error": "Forbidden"})),
        Err(e) => return Json(json!({"status": "error", "error": e.to_string()})),
    }

    let public_endpoint = env::var("MINIO_PUBLIC_ENDPOINT").expect("MINIO_PUBLIC_ENDPOINT not set");
    let minio = build_minio_client(&public_endpoint);
    match minio
        .presigned_get_object(PresignedArgs::new("bucket", &params.object_key).expires(3600))
        .await
    {
        Ok(url) => Json(json!({"status": "success", "url": url})),
        Err(e) => Json(json!({"status": "error", "error": e.to_string()})),
    }
}
