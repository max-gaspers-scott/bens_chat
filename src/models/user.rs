#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
}
