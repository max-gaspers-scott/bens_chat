#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ConversationParticipant {
    pub participant_id: uuid::Uuid,
    pub conversation_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}
