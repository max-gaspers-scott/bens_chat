use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct FetchUrlQuery {
    pub object_key: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateChatRequest {
    pub chat_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewPass {
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct UsernameQuery {
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct ParentQuery {
    pub parent: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UploadUrlQuery {
    pub chat_id: Uuid,
    pub file_extension: String,
}
