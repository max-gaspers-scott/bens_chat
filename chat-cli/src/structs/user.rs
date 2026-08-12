#[derive(serde::Serialize)]
pub struct User {
    pub name: String,
    pub phone_number: Option<String>,
    pub email: Option<String>,
    pub password_hash: String,
}
