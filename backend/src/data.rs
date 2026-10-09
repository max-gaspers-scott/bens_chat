use crate::auth::*;
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
use bens_chat_shared::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use socketioxide::{
    SocketIo,
    extract::{Data, SocketRef},
};
use sqlx::{PgPool, postgres::PgPoolOptions};
pub async fn post_nested_message(
    auth_user: &AuthUser,
    payload: &Message,
    pool: &PgPool,
) -> Result<Message, sqlx::Error> {
    let query =
        "INSERT INTO messages (sender_name, parent_id, content) VALUES ($1, $2, $3) RETURNING *";

    let q = sqlx::query_as::<_, Message>(&query)
        .bind(auth_user.username.clone())
        .bind(payload.parent_id.clone())
        .bind(payload.content.clone());

    q.fetch_one(pool).await
}

// a message impliments  #[derive(Debug, Clone, Serialize,
// Deserialize)], but  Value dosne yet and the compiler expects both values or resut to be serializable
pub async fn post_root_message(
    auth_user: &AuthUser,
    payload: &Message,
    pool: &PgPool,
) -> Result<Message, Json<Value>> {
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Err(Json(json!({"res": format!("error: {}", e)}))),
    };

    let message_query =
        "INSERT INTO messages (sender_name, parent_id, content) VALUES ($1, NULL, $2) RETURNING *";
    let message_result = sqlx::query_as::<_, Message>(&message_query)
        .bind(&auth_user.username)
        .bind(&payload.content)
        .fetch_one(&mut *tx)
        .await;

    let message = match message_result {
        Ok(m) => m,
        Err(e) => {
            let _ = tx.rollback().await;
            return Err(Json(json!({"res": format!("error: {}", e)})));
        }
    };

    //TODO: get rid of the concepts of chats from the DB entirely. a 1 to 1 onto mapping of
    //message id to another uuid is not helpful
    let chat_id = message.message_id;

    let participant_query = "INSERT INTO chat_participants (chat_id, user_name) VALUES ($1, $2)";
    if let Err(e) = sqlx::query(&participant_query)
        .bind(chat_id)
        .bind(&auth_user.username)
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        return Err(Json(json!({"res": format!("error: {}", e)})));
    }

    if let Err(e) = tx.commit().await {
        return Err(Json(json!({"res": format!("error: {}", e)})));
    }

    Ok(message)
}
