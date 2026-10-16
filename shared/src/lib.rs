use reqwest::{self, Client, Request};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

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
// use chrono::prelude::Utc;
// impl Message {
//     pub fn default(self) -> Message {
//         Message {
//             message_id: uuid::Uuid::new_v4(),
//             sender_name: "default".to_string(),
//             parent_id: Some(uuid::Uuid::new_v4()),
//             content: serde_json::json!({"text": "default".to_string()}),
//             sent_at: Utc::now(),
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_message_sender() {
        let parent_uuid = uuid::Uuid::new_v4();
        let user = "testUser".to_string();
        let content = serde_json::json!({
            "test": message,
        });
        let create_message = Message::create_message_sender(parent_id, user_name);
        let message1 = create_message(content);
        assert_eq!(message1.sender_name, user);
    }
}

/// The body sent from the CLI when posting a new message.
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessage {
    pub sender_name: String,
    pub parent_id: Option<uuid::Uuid>,
    pub content: serde_json::Value,
}

impl SendMessage {
    pub async fn send(&self, token: &str, url: &str) -> Result<uuid::Uuid, reqwest::Error> {
        let client = reqwest::Client::new();
        let responce = client
            .post(url)
            .json(&self.content)
            .bearer_auth(token)
            .send()
            .await?;
        let parsed_res = responce.json::<Uuid>().await?;
        Ok(parsed_res)
    }
    // love curry
    pub fn create_message_sender(
        parent_id: uuid::Uuid,
        user_name: &str,
    ) -> impl Fn(serde_json::Value) -> SendMessage {
        move |content: serde_json::Value| SendMessage {
            sender_name: user_name.to_string(),
            parent_id: Some(parent_id),
            content: content,
        }
    }
}

// async fn send_message(
//     login: &LoginPayload,
//     message: &SendMesage,
// ) -> Result<PostMsgRes, reqwest::Error> {
//     let url = format!("{BASE_URL}/messages");
//     let client = reqwest::Client::new();
//
//     let response = client
//         .post(url)
//         .json(message)
//         .bearer_auth(login.token.clone())
//         .send()
//         .await?;
//
//     let parsed_res = response.json::<PostMsgRes>().await?;
//     Ok(parsed_res)
// }

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
    #[serde(default)]
    pub player1: String,
    #[serde(default)]
    pub player2: String,
    pub name: String,
    pub grid: Vec<Col>,
    pub winner: Winner,
}

impl Connect4 {
    pub fn new(name: String, player1: String, player2: String) -> Connect4 {
        let board = vec![Col::new(); 7];
        Connect4 {
            name,
            player1: player1.clone(),
            player2: player2,
            grid: board,
            turn: Chip::Red,
            winner: Winner::GameStillGoing,
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

    // Helper to check for 4 consecutive chips in a given direction
    fn check_direction(&self, r: usize, c: usize, dr: isize, dc: isize) -> bool {
        let current_chip = match self.grid.get(c).and_then(|col| col.row.get(r)) {
            Some(chip) => chip,
            None => return false,
        };

        let mut count = 0;
        // Check in one direction (e.g., right, up, diagonal)
        for i in 0..4 {
            let new_r = (r as isize + i * dr);
            let new_c = (c as isize + i * dc);

            if new_r < 0 || new_c < 0 || new_c >= self.grid.len() as isize {
                break; // Out of bounds
            }
            let new_r = new_r as usize;
            let new_c = new_c as usize;

            if let Some(col) = self.grid.get(new_c) {
                if let Some(chip) = col.row.get(new_r) {
                    if chip == current_chip {
                        count += 1;
                    } else {
                        break; // Chips don't match
                    }
                } else {
                    break; // Out of bounds in row
                }
            } else {
                break; // Out of bounds in column
            }
        }
        if count >= 4 {
            return true;
        }

        // Check in the opposite direction (e.g., left, down, opposite diagonal)
        count = 0;
        for i in 1..4 {
            let new_r = (r as isize - i * dr);
            let new_c = (c as isize - i * dc);

            if new_r < 0 || new_c < 0 || new_c >= self.grid.len() as isize {
                break; // Out of bounds
            }
            let new_r = new_r as usize;
            let new_c = new_c as usize;

            if let Some(col) = self.grid.get(new_c) {
                if let Some(chip) = col.row.get(new_r) {
                    if chip == current_chip {
                        count += 1;
                    } else {
                        break; // Chips don't match
                    }
                } else {
                    break; // Out of bounds in row
                }
            } else {
                break; // Out of bounds in column
            }
        }
        count >= 3 // If initial chip + 3 in opposite direction == 4
    }

    // Checks for a win condition after a chip is dropped at (r, c)
    pub fn is_winner(&self, r: usize, c: usize) -> bool {
        // Check horizontal
        if self.check_direction(r, c, 0, 1) {
            return true;
        }
        // Check vertical
        if self.check_direction(r, c, 1, 0) {
            return true;
        }
        // Check diagonal (top-left to bottom-right)
        if self.check_direction(r, c, 1, 1) {
            return true;
        }
        // Check diagonal (top-right to bottom-left)
        if self.check_direction(r, c, 1, -1) {
            return true;
        }

        false
    }

    pub fn update(&self, pos: usize, player_name: String) -> Connect4 {
        let player_name = player_name.clone();
        let mut new_stat = self.clone();
        let is_plaer1 = player_name == new_stat.player1;
        let players_color = if is_plaer1 { Chip::Red } else { Chip::Yellow };
        let can_play = players_color == new_stat.turn;
        println!("player name: {player_name}");
        println!("plaery1: {:?}", self.player1);
        println!("player collor: {:?}", players_color);
        println!("can play: {can_play}");

        let t = match self.turn {
            Chip::Red => "red",
            Chip::Yellow => "yello",
        };
        if !can_play {
            return new_stat;
        }
        println!("current turn: {t}");
        new_stat.grid[pos].row.push(new_stat.turn.clone());
        let r = new_stat.grid[pos].row.len() - 1; // Correct row index

        if new_stat.is_winner(r, pos) {
            if is_plaer1 {
                new_stat.winner = Winner::Player1;
            } else {
                new_stat.winner = Winner::Player2;
            }
        }
        new_stat.switch_turn()
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Winner {
    Player1,
    Player2,
    GameStillGoing,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Chip {
    Red,
    Yellow,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Position {
    pub content: usize,
}
