#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Message {
    #[serde(default)]
    pub message_id: uuid::Uuid,
    pub sender_name: String,
    pub parent: Option<uuid::Uuid>,
    pub content: serde_json::Value,
    #[serde(default)]
    pub sent_at: chrono::DateTime<chrono::Utc>,
}
