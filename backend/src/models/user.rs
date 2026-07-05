#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub name: String,
    pub phone_number: Option<String>,
    pub email: Option<String>,
    pub password_hash: String,
}
