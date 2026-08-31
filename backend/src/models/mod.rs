// Types shared with the CLI — defined once in bens-chat-shared.
pub use bens_chat_shared::{Message, Note, User};

// Backend-only request/response types (query params, request bodies, etc.)
pub mod misc;
pub use misc::*;

// Legacy types still referenced in main.rs — kept local until main.rs is updated.
pub mod chat;
pub use chat::*;
pub mod new_chat_participant;
pub use new_chat_participant::*;
