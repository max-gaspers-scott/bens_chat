mod auth;
mod data;
mod models;
mod routes_serve;

use axum::{
    Extension,
    http::{
        HeaderValue, Method, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    response::{Html, IntoResponse},
};
use socketioxide::{
    SocketIo,
    extract::{Data, SocketRef},
};
use sqlx::postgres::PgPoolOptions;
use std::{convert::Infallible, env, result::Result};
use tower::service_fn;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    services::ServeDir,
};
use uuid::Uuid;

mod axum_handelers;

fn build_cors_layer() -> CorsLayer {
    let configured_origins = env::var("CORS_ALLOWED_ORIGINS").ok();

    let base = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any)
        .expose_headers([AUTHORIZATION, CONTENT_TYPE].map(|h| h));

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
// #[debug_handler]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("backend starting");
    // Try .env in the current directory first, then fall back to the workspace root (../)
    // so `cargo run` works from both the repo root and the backend/ subdirectory.
    if dotenv::dotenv().is_err() {
        dotenv::from_path("../.env").ok();
    }
    let (socket_layer, io) = SocketIo::new_layer();

    let db_url = env::var("DATABASE_URL").expect("no db url in env");
    // debug
    println!("url: {}", db_url);

    let pool = PgPoolOptions::new()
        .max_connections(100)
        .connect(&db_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let pool_for_ns = pool.clone();
    io.ns("/", move |s: SocketRef| {
        let pool = pool_for_ns.clone();
        println!("New socket connected: {:?}", s.id);
        let pool_clone = pool.clone();

        // Extract username from auth handshake / headers if available, or listen to a join event
        s.on(
            "join_chats",
            move |s: SocketRef, Data(username): Data<String>| {
                let pool = pool_clone.clone();
                tokio::spawn(async move {
                    match sqlx::query_scalar::<_, Uuid>(
                        "SELECT chat_id FROM chat_participants WHERE user_name = $1",
                    )
                    .bind(username)
                    .fetch_all(&pool)
                    .await
                    {
                        Ok(chat_ids) => {
                            for chat_id in chat_ids {
                                s.join(chat_id.to_string()).ok();
                            }
                            println!("Socket joined all chat rooms for user");
                        }
                        Err(e) => {
                            println!("Error fetching user chats for socket join: {e}");
                        }
                    }
                });
            },
        );

        s.on(
            "message",
            |s: SocketRef, Data(data): Data<serde_json::Value>| {
                println!("message received from FE: {:?}", data);
                s.broadcast()
                    .emit("message back", format!("echo: {:?}", data))
                    .ok();
            },
        )
    });

    // Fully custom migration runner that bypasses strict SQLx checksum validation

    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| "../frontend/build".to_string());
    let static_dir_path = std::path::PathBuf::from(&static_dir);

    let static_service = ServeDir::new(&static_dir)
        .append_index_html_on_directories(true)
        .not_found_service(service_fn(move |_req| {
            let index_path = static_dir_path.join("index.html");
            async move {
                let response = match tokio::fs::read_to_string(&index_path).await {
                    Ok(body) => (StatusCode::OK, Html(body)).into_response(),
                    Err(err) => {
                        let full_path = std::env::current_dir()
                            .map(|p| p.join(&index_path))
                            .unwrap_or(index_path.clone());
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to read index.html at {:?}: {}", full_path, err),
                        )
                            .into_response()
                    }
                };
                // Explicitly define the error type as Infallible.
                Ok::<_, Infallible>(response)
            }
        }));

    let app = routes_serve::routes()
        .fallback_service(static_service) // 1. Register fallback first
        .layer(Extension(io))
        .layer(socket_layer)
        .layer(build_cors_layer())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
