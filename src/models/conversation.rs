#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Conversation {
    pub conversation_id: uuid::Uuid,
    pub name: Option<String>,
    pub is_group_chat: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
