#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ChatParticipant {
    pub chat_participant_id: uuid::Uuid,
    pub chat_id: uuid::Uuid,
    pub user_name: String,
}
