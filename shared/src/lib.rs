use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// User
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct User {
    pub name: String,
    pub phone_number: Option<String>,
    pub email: Option<String>,
    pub password_hash: String,
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

/// A message row. `C` is the content type:
///   - backend uses `serde_json::Value` (raw JSONB from the DB)
///   - CLI uses `SendableContent` (typed, deserialized variant)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Message<C = serde_json::Value> {
    #[serde(default)]
    pub message_id: uuid::Uuid,
    pub sender_name: String,
    pub parent_id: Option<uuid::Uuid>,
    pub content: C,
    #[serde(default)]
    pub sent_at: chrono::DateTime<chrono::Utc>,
}

/// The body sent from the CLI when posting a new message.
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessage {
    pub sender_name: String,
    pub parent_id: Option<uuid::Uuid>,
    pub content: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Note
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Note {
    pub note_id: uuid::Uuid,
    pub text: String,
    pub refers_to_user_name: Option<String>,
    pub created_by_user_name: String,
    pub contact_name: String,
}

// ---------------------------------------------------------------------------
// SendableContent — the typed variants that can live inside Message.content
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SendableContent {
    Img(ImgMessage),
    Text(TextMessage),
    Title(TitleMessage),
    Con4(Connect4),
}

impl SendableContent {
    pub fn get_content(&self) -> String {
        match self {
            Self::Text(t) => t.text.clone(),
            Self::Img(i) => i.url.clone(),
            Self::Title(t) => t.title.clone(),
            Self::Con4(b) => b.name.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextMessage {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TitleMessage {
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImgMessage {
    pub url: String,
}

// ---------------------------------------------------------------------------
// Connect4
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Connect4 {
    pub turn: Chip,
    pub name: String,
    pub grid: Vec<Col>,
}

impl Connect4 {
    pub fn new(name: String, pos: usize) -> Connect4 {
        let mut begin = vec![Col::new(); pos - 1];
        begin.push(Col::new_start(Chip::Red)); // red always starts
        let end = vec![Col::new(); 7 - pos];
        let total = [begin, end].concat();
        Connect4 {
            name,
            grid: total,
            turn: Chip::Red,
        }
    }
    fn switch_turn(self) -> Connect4 {
        let new_board = self.clone();
        let new_turn = match self.turn {
            Chip::Red => Chip::Yellow,
            Chip::Yellow => Chip::Red,
        };
        Connect4 {
            turn: new_turn,
            ..self
        }
    }
    pub fn update(&self, pos: usize) -> Connect4 {
        let new_stat = self.clone();
        let mut new_stat = new_stat.switch_turn();
        new_stat.grid[pos].row.push(new_stat.turn.clone());
        new_stat
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Col {
    pub row: Vec<Chip>,
}

impl Col {
    pub fn new() -> Col {
        Col { row: Vec::new() }
    }
    pub fn new_start(chip: Chip) -> Col {
        Col { row: vec![chip] }
    }
}

impl Default for Col {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Chip {
    Red,
    Yellow,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Position {
    pub content: usize,
}
