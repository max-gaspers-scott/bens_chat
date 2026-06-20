#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Chat {
    pub chat_id: uuid::Uuid,
    pub root_message_id: uuid::Uuid,
}
