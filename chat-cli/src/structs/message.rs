#[derive(Debug, serde::Deserialize)]
pub struct Message {
    #[serde(default)]
    pub message_id: uuid::Uuid,
    pub sender_name: String,
    pub parent: Option<uuid::Uuid>,
    pub content: SendibleContent,
    #[serde(default)]
    pub sent_at: chrono::DateTime<chrono::Utc>,
}
