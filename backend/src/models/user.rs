#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub phone_number: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}
