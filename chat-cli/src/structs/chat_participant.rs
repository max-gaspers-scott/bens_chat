#[derive(serde::Serialize)]
pub struct ChatParticipant {
    pub chat_id: uuid::Uuid,
    pub user_name: String,
}
