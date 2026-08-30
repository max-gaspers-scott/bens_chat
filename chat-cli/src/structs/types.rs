use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct MessageResponce {
    pub payload: Vec<Message>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub message_id: uuid::Uuid,
    pub sender_name: String,
    pub parent_id: Option<uuid::Uuid>,
    pub content: SendibleContent,
    #[serde(default)]
    pub sent_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SendibleContent {
    Img(ImgMessage),
    Text(TextMessage),
    Title(TitleMessage),
    Con4(Connect4),
}

impl SendibleContent {
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
pub struct SendMesage {
    pub sender_name: String,
    pub parent_id: Option<uuid::Uuid>,
    pub content: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TextMessage {
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TitleMessage {
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImgMessage {
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Connect4 {
    pub name: String,
    pub grid: Vec<Col>,
}

impl Connect4 {
    pub fn new(name: String, pos: usize) -> Connect4 {
        let mut begin = vec![Col::new(); pos - 1];
        begin.push(Col::new_start(Chip::Red)); // red always starts
        let end = vec![Col::new(); 7 - pos];
        let total = [begin, end].concat();
        Connect4 { name, grid: total }
    }

    pub fn update(old_board: Connect4, _pos: usize) -> Connect4 {
        Connect4 {
            name: old_board.name,
            grid: old_board.grid,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Chip {
    Red,
    Yellow,
}
