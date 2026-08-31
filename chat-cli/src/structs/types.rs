// Re-export shared types the CLI uses directly.
pub use bens_chat_shared::{
    Chip, Col, Connect4, ImgMessage, SendMessage, SendableContent, TextMessage, TitleMessage, User,
};

// Backwards-compat alias: the CLI historically called this SendMesage (one 's').
pub use bens_chat_shared::SendMessage as SendMesage;

/// The CLI's typed message: content is deserialized into SendableContent variants.
pub type Message = bens_chat_shared::Message<SendableContent>;

/// CLI-only wrapper returned by the /messages and /user-chats endpoints.
#[derive(Debug, serde::Deserialize)]
pub struct MessageResponce {
    pub payload: Vec<Message>,
    pub status: String,
}
