#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct UserChat {
    pub user_id: uuid::Uuid,
    pub chat_id: uuid::Uuid,
}
