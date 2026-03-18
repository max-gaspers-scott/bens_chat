#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub message_id: uuid::Uuid,
    pub chat_id: uuid::Uuid,
    pub sender_id: uuid::Uuid,
    pub content: String,
    pub sent_at: Option<chrono::DateTime<chrono::Utc>>,
}
