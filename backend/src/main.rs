// minio stuff
use dotenv::dotenv;
use minio_rsc::Minio;
use minio_rsc::client::PresignedArgs;
use minio_rsc::provider::StaticProvider;
use reqwest::header::{ACCEPT, CONTENT_TYPE as CT};
use serde::{Deserialize, Serialize};

mod auth;
mod models;

use crate::{auth::AuthUser, models::*};
use axum::{
    Extension, Json, Router,
    extract::{self, Query, Request},
    http::{
        HeaderValue, Method, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
        request,
    },
    middleware,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use bcrypt::{DEFAULT_COST, hash, verify};
use core::str;
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{env, result::Result};
use tower::service_fn;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    services::ServeDir,
};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct FetchUrlQuery {
    object_key: String,
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

#[derive(Debug, Deserialize)]
struct NewPass {
    new_password: String,
}

#[derive(Debug, Deserialize)]
struct UsernameQuery {
    username: String,
}
#[derive(Debug, Deserialize)]
struct ParentQuery {
    parent: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
struct UploadUrlQuery {
    chat_id: Uuid,
    file_extension: String,
}
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

    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| "../frontend/build".to_string());
    let static_dir_path = std::path::PathBuf::from(&static_dir);

    let static_service = ServeDir::new(&static_dir)
        .append_index_html_on_directories(true)
        .not_found_service(service_fn(move |_req| {
            let index_path = static_dir_path.join("index.html");
            async move {
                match tokio::fs::read_to_string(&index_path).await {
                    Ok(body) => Ok((StatusCode::OK, Html(body)).into_response()),
                    Err(err) => {
                        let full_path = std::env::current_dir()
                            .map(|p| p.join(&index_path))
                            .unwrap_or(index_path.clone());
                        Ok((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to read index.html at {:?}: {}", full_path, err),
                        )
                            .into_response())
                    }
                }
            }
        }));

    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/users", post(post_user))
        .route("/auth/login", post(login_user));
    // .route("/debug-headers", get(debug_headers));

    let protected_routes = Router::new()
        .route(
            "/user-chats",
            get(post_chat_participant).post(post_chat_participant), //is
                                                                    // the get part nessisary
        )
        .route(
            "/messages",
            get(get_message_id_sender_name_content_parent).post(post_message),
        )
        // .route("/users", get(get_user_id_username))
        // .route("/chats", post(post_chat)) // no more chats, so just post mesages
        //
        .route("/password-set", post(set_password))
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
async fn is_user_in_chat(pool: &PgPool, name: String, chat_id: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_chats WHERE user_id = $1 AND chat_id = $2)",
    )
    .bind(name)
    .bind(chat_id)
    .fetch_one(pool)
    .await
}
async fn set_password(
    extract::State(pool): extract::State<PgPool>,
    Extension(auth_user): Extension<AuthUser>,
    Query(new_pass): Query<NewPass>,
) -> Json<Value> {
    let password_hash = match hash(&new_pass.new_password, DEFAULT_COST) {
        Ok(password_hash) => password_hash,
        Err(e) => {
            return Json(json!({"status": "error", "error": format!("Hash error: {}", e)}));
        }
    };

    let result = sqlx::query("UPDATE users SET password_hash = $1 WHERE user_name = $2")
        .bind(password_hash)
        .bind(auth_user.username)
        .execute(&pool)
        .await;

    match result {
        Ok(_) => Json(json!({"status": "success"})),
        Err(e) => Json(json!({"status": "error", "error": e.to_string()})),
    }
}
async fn get_message_id_sender_name_content_parent(
    Extension(auth_user): Extension<AuthUser>,
    match_val: Query<ParentQuery>,
    extract::State(pool): extract::State<PgPool>,
) -> Json<Value> {
    let query = "SELECT * FROM messages WHERE parent = $1";
    let q = sqlx::query_as::<_, Message>(&query).bind(match_val.parent.clone());

    match q.fetch_all(&pool).await {
        Ok(messages) => {
            let payloads: Vec<Value> = messages
                .into_iter()
                .map(|m| {
                    json!({
                        "message_id": m.message_id,
                        "sender_name": m.sender_name,
                        "content": m.content,
                        "sent_at": m.sent_at,
                    })
                })
                .collect();
            Json(json!({
                "status": "success",
                "payload": payloads
            }))
        }
        Err(e) => Json(json!({
            "status": "error",
            "error": format!("Database error: {}", e)
        })),
    }
}

//TODO: @gemini is bad solution
//should check if the chat is with a user named "gemini" and no other users and if so post gemini
//respoce
pub async fn post_message(
    Extension(auth_user): Extension<AuthUser>,
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<Message>,
) -> Json<Value> {
    // if first_word == "@gemini" {
    //     println!("message starts with @gemini");
    //     let gem_res = match gemini(&value.content).await {
    //         Ok(res) => res,
    //         Err(e) => format!("Error generating content: {}", e),
    //     };
    //     let _ = sqlx::query_as::<_, Message>(
    //         "INSERT INTO messages (chat_id, sender_id, content, minio_url) VALUES ($1, $2, $3, $4) RETURNING *",
    //     )
    //     .bind(payload.chat_id)
    //     .bind(auth_user.user_id) // why not gemini a
    //         // hardcoded uuid of gemini??
    //     .bind(gem_res)
    //     .bind(payload.minio_url)
    //     .fetch_one(&pool)
    //     .await;
    // }
    // change hardcoded number of values
    let query =
        "INSERT INTO messages (sender_name, parent, content) VALUES ($1, $2, $3) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, Message>(&query)
        .bind(auth_user.username)
        .bind(payload.parent)
        .bind(payload.content);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}
#[derive(Deserialize, Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Deserialize, Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Deserialize, Debug, Serialize)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Deserialize, Debug, Serialize)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Deserialize, Debug, Serialize)]
struct PartResponse {
    text: String,
}

#[derive(Deserialize, Debug, Serialize)]
struct GenerateContentResponse {
    contents: Vec<Content>,
}

#[derive(Deserialize, Debug, Serialize)]
struct GeminiRespons {
    candidates: Vec<Candidate>,
}

async fn gemini(message: &str) -> Result<String, reqwest::Error> {
    dotenv().ok();
    let api_key_name = "GEMINI_API_KEY";
    let api_key: String = match env::var(api_key_name) {
        Ok(val) => val.trim().to_string(),
        Err(e) => {
            println!("couldn't interpret {api_key_name}: {e}");
            format!("{}", e)
        }
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );

    // 3. Construct the Request Body using the Serde structs
    let request_body = GenerateContentResponse {
        contents: vec![Content {
            parts: vec![Part {
                text: message.to_string(),
            }],
        }],
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header(CT, "application/json")
        .header(ACCEPT, "application/json")
        // reqwest::Client::post() automatically uses the body's Serialize implementation
        // and sets the Content-Length header when sending the request body.
        .json(&request_body)
        .send()
        .await?;

    let text = if response.status().is_success() {
        // Deserialize the JSON response into our Rust struct
        let json_response: GeminiRespons = response.json().await?;

        // TODO: should not return "" insted do better error handeling
        // program should not continue with empty string is somthing goes wrong at this step
        if let Some(candidate) = json_response.candidates.first() {
            if let Some(part) = candidate.content.parts.first() {
                part.text.to_string()
            } else {
                println!("could not get part.text from api");
                "".to_string()
            }
        } else {
            println!("Response was successful but had no candidates.");
            "".to_string()
        }
    } else {
        eprintln!("\n❌ API Request Failed!");
        eprintln!("Status: {}", response.status());
        eprintln!("Body: {}", response.text().await?);
        "".to_string()
    };

    println!("Generated text: {}", text);
    Ok(text)
}

pub async fn post_user(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<User>,
) -> Json<Value> {
    let password_hash = match hash(&payload.password_hash, 12) {
        Ok(password_hash) => password_hash,
        Err(e) => return Json(json!({"res": format!("error: {}", e)})),
    };
    // change hardcoded number of values
    let query = "INSERT INTO users (name, phone_number, email, passwrod_hash) VALUES ($1, $2, $3, $4) RETURNING name, phone_number, email, passwrod_hash AS password_hash";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, User>(&query)
        .bind(payload.name)
        .bind(payload.phone_number)
        .bind(payload.email)
        .bind(password_hash);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
}

async fn login_user(
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Json<Value> {
    let result = sqlx::query_as::<_, User>("SELECT name, phone_number, email, passwrod_hash AS password_hash FROM users WHERE name = $1")
        .bind(&payload.username)
        .fetch_optional(&pool)
        .await;

    match result {
        Ok(Some(user)) if verify(&payload.password, &user.password_hash).unwrap_or(false) => {
            match auth::create_token(&user.name) {
                Ok(token) => Json(json!({
                    "status": "success",
                    "payload": {
                        "username": user.name,
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

// async fn post_user_chat(
//     Extension(auth_user): Extension<AuthUser>,
//     extract::State(pool): extract::State<PgPool>,
//     Json(payload): Json<UserChat>,
// ) -> Json<Value> {
//     if payload.user_id != auth_user.user_id {
//         match is_user_in_chat(&pool, auth_user.user_id, payload.chat_id).await {
//             Ok(true) => {}
//             Ok(false) => return Json(json!({"res": "error: forbidden"})),
//             Err(e) => return Json(json!({"res": format!("error: {}", e)})),
//         }
//     }
//
//     let result = sqlx::query_as::<_, UserChat>(
//         "INSERT INTO user_chats (user_id, chat_id) VALUES ($1, $2) RETURNING *",
//     )
//     .bind(payload.user_id)
//     .bind(payload.chat_id)
//     .fetch_one(&pool)
//     .await;
//
//     match result {
//         Ok(value) => Json(json!({"res": "success", "data": value})),
//         Err(e) => Json(json!({"res": format!("error: {}", e)})),
//     }
// }
pub async fn post_chat_participant(
    Extension(auth_user): Extension<AuthUser>,
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<ChatParticipant>,
) -> Json<Value> {
    // change hardcoded number of values
    let query = "INSERT INTO chat_participants (chat_id, user_name) VALUES ($1, $2) RETURNING *";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, ChatParticipant>(&query)
        .bind(payload.chat_id)
        .bind(payload.user_name);

    let result = q.fetch_one(&pool).await;

    match result {
        Ok(value) => Json(json!({"res": "success", "data": value})),
        Err(e) => Json(json!({"res": format!("error: {}", e)})),
    }
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
                "username": user.name,
            }
        })),
        Ok(None) => Json(json!({"status": "error", "error": "User not found"})),
        Err(e) => Json(json!({"status": "error", "error": e.to_string()})),
    }
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
    match is_user_in_chat(&pool, auth_user.username, params.chat_id).await {
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
    match is_user_in_chat(&pool, auth_user.username, chat_id).await {
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
