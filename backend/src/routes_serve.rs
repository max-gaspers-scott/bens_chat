use axum::{
    Extension, Json, Router,
    extract::{self, Query},
    middleware,
    routing::{get, post},
};
use bens_chat_shared::{Connect4, Position};
use bcrypt::{DEFAULT_COST, hash, verify};
use minio_rsc::client::PresignedArgs;
use minio_rsc::provider::StaticProvider;
use minio_rsc::Minio;
use serde::Deserialize;
use socketioxide::SocketIo;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{env, result::Result};
use uuid::Uuid;

use crate::{auth, auth::AuthUser, data, models::*};

pub fn routes() -> Router<PgPool> {
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/users", post(post_user))
        .route("/test", get(|| async { "hi fe" }))
        .route("/auth/login", post(login_user));

    // .route("/debug-headers", get(debug_headers));

    let protected_routes = Router::new()
        .route(
            "/user-chats",
            get(get_users_chats).post(post_chat_participant), // need
                                                              // a real get part so users can see there "chats"
        )
        .route(
            "/messages",
            get(get_message_id_sender_name_content_parent)
                .post(post_message)
                // .patch(update_message)
                .patch(update_connect),
        )
        .route("/password-set", post(set_password))
        .route("/minio-fetch", get(get_fetch_url))
        .route("/minio-post", get(get_put_url))
        .layer(middleware::from_fn(auth::authorize));

    Router::new().merge(public_routes).merge(protected_routes)
}

//
//
// !!!!!!!!!!!!!!!!   handlers    !!!!!!!!!!!!!!!!
//
//
//
//
//
//
//

async fn health() -> String {
    "healthy".to_string()
}
async fn update_connect(
    extract::State(pool): extract::State<PgPool>,
    Extension(auth_user): Extension<AuthUser>,
    Query(name): Query<MessageName>,
    Json(positsion): Json<Position>,
) -> Json<Value> {
    let query = r#"
      SELECT *
      FROM messages
      WHERE content->>'name' = $1
      ORDER BY sent_at DESC
      LIMIT 1;
     "#;
    let result = sqlx::query_as::<_, Message>(query)
        .bind(&name.name)
        .fetch_one(&pool)
        .await;
    let msg = match result {
        Ok(r) => r,
        Err(_e) => {
            panic!("error geting message")
        }
    };
    let board = msg.content;
    let mut board_value = board.clone();
    if let Some(winner_entry) = board_value.get_mut("winner") {
        if winner_entry.is_null() {
            *winner_entry = serde_json::Value::String("GameStillGoing".to_string());
        }
    } else {
        board_value["winner"] = serde_json::Value::String("GameStillGoing".to_string());
    }

    let board = match serde_json::from_value::<Connect4>(board_value) {
        Ok(c) => c,
        Err(e) => {
            panic!("error converting to Conenct4: {}", e);
        }
    };
    let new_board = board.update(positsion.content, auth_user.username.clone());
    let new_board_json = serde_json::to_value(&new_board).unwrap();

    let query = r#"
 UPDATE messages
  SET content = $1
  WHERE message_id = (
      SELECT message_id
      FROM messages
      WHERE content->>'name' = $2
      ORDER BY sent_at DESC
      LIMIT 1
  );
"#;
    let result = sqlx::query(query)
        .bind(new_board_json)
        .bind(&name.name)
        .bind(auth_user.username)
        .execute(&pool)
        .await;
    match result {
        Ok(_r) => Json(json!({ "status": "success" })),
        Err(e) => {
            println!("update error: {e}");
            Json(json!({ "status": "error" }))
        }
    }
}

#[derive(Debug, Deserialize)]
struct MessageName {
    name: String,
}
#[derive(Debug, Deserialize)]
struct UpdateMessage {
    content: serde_json::Value,
}
async fn update_message(
    extract::State(pool): extract::State<PgPool>,
    Extension(auth_user): Extension<AuthUser>,
    Query(name): Query<MessageName>,
    Json(content): Json<UpdateMessage>,
) -> Json<Value> {
    //TODO: should not be all palces where name = that-name,
    //it should be jsut the most resent message with that naem.
    //should probaby order by date first
    println!("inside update func");

    // may want this but not for gameboards
    //       AND sender_name = $3
    let query = r#"
 UPDATE messages
  SET content = $1
  WHERE message_id = (
      SELECT message_id
      FROM messages
      WHERE content->>'name' = $2
      ORDER BY sent_at DESC
      LIMIT 1
  );
"#;

    let result = sqlx::query(query)
        .bind(content.content)
        .bind(&name.name)
        .bind(auth_user.username)
        .execute(&pool)
        .await;
    match result {
        Ok(_r) => Json(json!({ "status": "success" })),
        Err(e) => {
            println!("update error: {e}");
            Json(json!({ "status": "error" }))
        }
    }
}

async fn get_users_chats(
    extract::State(pool): extract::State<PgPool>,
    Extension(_auth_user): Extension<AuthUser>,
    Query(username): Query<UsernameQuery>,
) -> Json<Value> {
    let query = "SELECT m.message_id, m.sender_name, m.parent_id, m.content, m.sent_at \
                 FROM messages m \
                 JOIN chat_participants cp ON cp.chat_id = m.message_id \
                 WHERE cp.user_name = $1 AND m.parent_id IS NULL";
    let q = sqlx::query_as::<_, Message>(query).bind(username.username.clone());

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

async fn is_user_in_chat(pool: &PgPool, name: &str, chat_id: &Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        r#"
        WITH RECURSIVE chat_tree AS (
            SELECT
                m.message_id,
                m.parent_id,
                m.message_id as original_message_id
            FROM messages m
            WHERE m.message_id = $2 -- Start with the given message_id

            UNION ALL

            SELECT
                m_rec.message_id,
                m_rec.parent_id,
                ct.original_message_id
            FROM messages m_rec
            JOIN chat_tree ct ON m_rec.message_id = ct.parent_id
        )
        SELECT EXISTS (
            SELECT 1
            FROM chat_participants cp
            JOIN (
                WITH RECURSIVE chat_ancestry AS (
                    SELECT message_id, parent_id
                    FROM messages
                    WHERE message_id = $2
                    UNION ALL
                    SELECT m.message_id, m.parent_id
                    FROM messages m
                    JOIN chat_ancestry ca ON m.message_id = ca.parent_id
                )
                SELECT message_id
                FROM chat_ancestry
                WHERE parent_id IS NULL
            ) AS root_message ON cp.chat_id = root_message.message_id
            WHERE cp.user_name = $1
        )
        "#,
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
    match is_user_in_chat(&pool, &auth_user.username, &match_val.parent).await {
        Ok(true) => {}
        Ok(false) => return Json(json!({"status": "error", "error": "Forbidden"})),
        Err(e) => return Json(json!({"status": "error", "error": e.to_string()})),
    }

    let query = "SELECT * FROM messages WHERE parent_id = $1 ORDER BY sent_at";
    let q = sqlx::query_as::<_, Message>(query).bind(match_val.parent);

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

//TODO: the function (and endpoint) has too much responsibility
//should have a separate endpoint+handler for posting root messages, maybe called create chat
//
async fn post_message(
    Extension(auth_user): Extension<AuthUser>,
    extract::State(pool): extract::State<PgPool>,
    Extension(io): Extension<SocketIo>,
    Json(payload): Json<Message>,
) -> Json<Value> {
    let _text = &payload
        .content
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("not_gemini");

    // change hardcoded number of values
    let _query =
        "INSERT INTO messages (sender_name, parent_id, content) VALUES ($1, $2, $3) RETURNING *";
    if let Some(_parent) = payload.parent_id {
        let result = data::post_nested_message(&auth_user, &payload, &pool).await;

        match result {
            Ok(ref value) => {
                let pool_clone = pool.clone();
                let io_clone = io.clone();
                let msg_val = json!({"res": "success", "data": value});
                let msg_id = value.message_id;
                let msg_val_clone = msg_val.clone();
                tokio::spawn(async move {
                    if let Ok(root_id) = get_root_chat_id(&pool_clone, msg_id).await {
                        io_clone
                            .to(root_id.to_string())
                            .emit("new_message", &msg_val_clone)
                            .ok();
                    }
                });
                Json(msg_val)
            }
            Err(e) => Json(json!({"res": format!("error: {}", e)})),
        }
    } else {
        let message = data::post_root_message(&auth_user, &payload, &pool).await;
        match message {
            Ok(_) => {
                let pool_clone = pool.clone();
                let io_clone = io.clone();
                let msg_val = json!({"res": "success", "data": message.clone().unwrap()});
                let msg_id = message.unwrap().message_id;
                let msg_val_clone = msg_val.clone();
                let value = msg_val_clone.clone();
                tokio::spawn(async move {
                    if let Ok(root_id) = get_root_chat_id(&pool_clone, msg_id).await {
                        io_clone
                            .to(root_id.to_string())
                            .emit("new_message", &value)
                            .ok();
                    }
                });
                Json::from(msg_val_clone)
            }
            Err(json) => json,
        }
    }
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
    let query = "INSERT INTO users (name, phone_number, email, password_hash) VALUES ($1, $2, $3, $4) RETURNING name, phone_number, email, password_hash";

    //// what is bound is wrong
    let q = sqlx::query_as::<_, User>(query)
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
    let result = sqlx::query_as::<_, User>(
        "SELECT name, phone_number, email, password_hash FROM users WHERE name = $1",
    )
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
pub async fn post_chat_participant(
    Extension(_auth_user): Extension<AuthUser>,
    extract::State(pool): extract::State<PgPool>,
    Json(payload): Json<NewChatParticipant>,
) -> Json<Value> {
    // change hardcoded number of values
    let query = "INSERT INTO chat_participants (chat_id, user_name) VALUES ($1, $2) RETURNING *";

    let q = sqlx::query_as::<_, NewChatParticipant>(query)
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
    let result = sqlx::query_as::<_, User>(
        "SELECT name, phone_number, email, password_hash FROM users WHERE name = $1",
    )
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
    match is_user_in_chat(&pool, &auth_user.username, &params.chat_id).await {
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
    match is_user_in_chat(&pool, &auth_user.username, &chat_id).await {
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

async fn get_root_chat_id(pool: &PgPool, message_id: Uuid) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        WITH RECURSIVE chat_ancestry AS (
            SELECT message_id, parent_id
            FROM messages
            WHERE message_id = $1
            UNION ALL
            SELECT m.message_id, m.parent_id
            FROM messages m
            JOIN chat_ancestry ca ON m.message_id = ca.parent_id
        )
        SELECT message_id
        FROM chat_ancestry
        WHERE parent_id IS NULL
        LIMIT 1
        "#,
    )
    .bind(message_id)
    .fetch_one(pool)
    .await
}
