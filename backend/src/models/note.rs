#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Note {
pub     note_id: uuid::Uuid,
pub     text: String,
pub     refers_to_user_name: Option<String>,
pub     created_by_user_name: String,
pub     contact_name: String,
}
