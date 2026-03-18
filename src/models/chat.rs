#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Chat {
    pub chat_id: uuid::Uuid,
    pub chat_name: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}
