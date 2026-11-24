//! bens chat shared
//! Use it but build yoru own frontend for bens chat or make a bot that uses it
//!
//! enjoy!
use reqwest::{self};
use serde::{Deserialize, Serialize};
use serde_json::json;
//for bot
use rust_socketio::{ClientBuilder, Payload, RawClient};
use std::thread;

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
//
// #[cfg(test)]
// mod tests {
// use super::*;
//     // for testing
//     #[derive(deserialize)]
//     struct loginresponse {
//         payload: loginpayload,
//         status: string,
//     }
//     #[derive(clone, deserialize, debug)]
//     struct loginpayload {
//         token: string,
//         username: string,
//     }
//     async fn get_jwt() -> string {
//         // let url = "localhost:9821/auth/login";
//         // let payload = serde_json::json!({
//         //     "username": "test0",
//         //     "password": "aaa",
//         // });
//         //
//         // let client = reqwest::client::new();
//         //
//         // let res = match client.post(url).json(&payload).s.awaitend().await {
//         //     ok(res) => res,
//         //     err(e) => {
//         //         println!("network or request error: {e}. please try again.");
//         //         panic!();
//         //     }
//         // };
//         //
//         // if !res.status().is_success() {
//         //     let status = res.status();
//         //     let body = res.text().await.unwrap_or_default();
//         //     println!(
//         //         "login failed (status {}): {}. please try again.",
//         //         status, body
//         //     );
//         //     panic!();
//         // }
//         // no work
//         string::from("test jwt")
//
//         // return ok(user_info);
//     }
//     #[test]
//     fn create_message_sender() {
//         let parent_uuid = uuid::uuid::new_v4();
//         let user = "testuser".to_string();
//
//         let content = serde_json::json!({
//             "test": "test message",
//         });
//         let create_message = Sendmessage::make_send_message_creater(parent_uuid, &user);
//         let message1 = create_message(content);
//         assert_eq!(message1.sender_name, user);
//     }
//
//     #[test]
//     fn send_message() {
//         let parent_uuid = uuid::uuid::new_v4();
//         let user = "testuser".to_string();
//
//         let content = serde_json::json!({
//             "test": "test message",
//         });
//         let create_message = sendmessage::make_send_message_creater(parent_uuid, &user);
//         let message1 = create_message(content);
//
//         let jwt = get_jwt().await;
//         message1
//             .send(jwt, "http://localhost:9821/messages")
//             .await
//             .unwrap()
//     }
// }

/// The response body from `POST /messages`:
/// `{"res": "success", "data": <Message>}` on success,
/// `{"res": "error: ..."}` on failure.
#[derive(Debug, Deserialize)]
struct PostMessageResponse {
    res: String,
    data: Option<Message>,
}

/// The body sent from the CLI when posting a new message.
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessage {
    pub sender_name: String,
    pub parent_id: Option<uuid::Uuid>,
    pub content: serde_json::Value,
}

impl SendMessage {
    pub async fn send(
        &self,
        token: String,
        url: String,
    ) -> Result<uuid::Uuid, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let responce = client
            .post(url)
            .json(self)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?;
        let parsed_res = responce.json::<PostMessageResponse>().await?;
        match parsed_res.data {
            Some(message) if parsed_res.res == "success" => Ok(message.message_id),
            _ => Err(format!("failed to send message: {}", parsed_res.res).into()),
        }
    }
    // love curry
    pub fn make_send_message_creater(
        parent_id: uuid::Uuid,
        user_name: String,
    ) -> impl Fn(serde_json::Value) -> SendMessage {
        move |content: serde_json::Value| SendMessage {
            sender_name: user_name.to_string(),
            parent_id: Some(parent_id),
            content,
        }
    }
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
    Unsuportad(UnsuportedMsg),
}

impl SendableContent {
    pub fn get_content(&self) -> String {
        match self {
            Self::Text(t) => t.text.clone(),
            Self::Img(i) => i.url.clone(),
            Self::Title(t) => t.title.clone(),
            Self::Con4(b) => b.name.clone(),
            Self::Unsuportad(u) => "unsuported message".to_string(),
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
// U
// ---------------------------------------------------------------------------
#[derive(Debug, Serialize, Deserialize)]
pub struct UnsuportedMsg {}

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
            player2,
            grid: board,
            turn: Chip::Red,
            winner: Winner::GameStillGoing,
        }
    }
    fn switch_turn(self) -> Connect4 {
        let _new_board = self.clone();
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
            let new_r = r as isize + i * dr;
            let new_c = c as isize + i * dc;

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
            let new_r = r as isize - i * dr;
            let new_c = c as isize - i * dc;

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

// name and passbe should  from env var
pub struct Bot {}

// const BASE_URL: &str = "https://bens-chat.team-stingray.com";
const BASE_URL: &str = "http://localhost:9821";
#[derive(Deserialize)]
struct LoginResponse {
    payload: LoginPayload,
    status: String,
}
#[derive(Clone, Deserialize, Debug)]
struct LoginPayload {
    token: String,
    username: String,
}
async fn jwt() -> String {
    let url = format!("{BASE_URL}/auth/login");
    let payload = serde_json::json!({
        "username": "t0",
        "password": "aaa",
    });

    let client = reqwest::blocking::Client::new();

    let res = match client.post(url).json(&payload).send() {
        Ok(res) => res,
        Err(e) => {
            println!("Network or request error: {e}. Please try again.");
            panic!();
        }
    };

    let text = match res.text() {
        Ok(text) => text,
        Err(e) => {
            println!("Failed to read response: {e}. Please try again.");
            panic!();
        }
    };

    let data: LoginResponse = match serde_json::from_str(&text) {
        Ok(data) => data,
        Err(e) => {
            println!("Could not parse login response ({e}). Please try again.");
            panic!();
        }
    };

    let user_info = data.payload;
    user_info.token
}

impl Bot {
    pub async fn start<F>(respond: F)
    where
        F: Fn(Message) -> SendMessage + Send + 'static,
    {
        let token = jwt().await;
        let callback = move |payload: Payload, _s: RawClient| {
            if let Payload::Text(values) = payload {
                for v in values {
                    let data = v["data"].clone();

                    let d: Message = serde_json::from_value(data).unwrap();
                    let res = respond(d.clone());
                    let msg = SendMessage {
                        sender_name: String::from("t0"),
                        parent_id: res.parent_id,
                        content: res.content,
                    };

                    let t = token.clone();
                    thread::spawn(async move || {
                        let _m = msg.send(t, String::from("http://localhost:9821")).await;
                    });
                }
            }
        };
        let _socket = ClientBuilder::new(BASE_URL)
            .namespace("/")
            .on("open", move |_, s: RawClient| {
                // TODO: change hard coded name
                if let Err(e) = s.emit("join_chats", json!("t0")) {
                    eprintln!("join failed: {e}");
                }
            })
            .on("new_message", callback)
            .on("error", |err, _| eprintln!("Error: {:#?}", err))
            .connect()
            .expect("Connection failed");

        // - will need username and passord to get jwt
        // pass mesage to fn
        // send back resulting messae to right chat
    }
    pub async fn no_responce<F>(udf: F)
    where
        F: Fn(Message) + Send + 'static,
    {
        // fire and forget udf (user defind function), using thread::spon so the udf can be an async
        let callback = move |payload: Payload, _s: RawClient| {
            if let Payload::Text(values) = payload {
                for v in values {
                    let data = v["data"].clone();

                    let d: Message = serde_json::from_value(data).unwrap();
                    udf(d.clone());
                }
            }
        };
        let _socket = ClientBuilder::new(BASE_URL)
            .namespace("/")
            .on("open", move |_, s: RawClient| {
                // TODO: change hard coded name
                if let Err(e) = s.emit("join_chats", json!("t0")) {
                    eprintln!("join failed: {e}");
                }
            })
            .on("new_message", callback)
            .on("error", |err, _| eprintln!("Error: {:#?}", err))
            .connect()
            .expect("Connection failed");
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot() {
        let test_func = |msg: Message| SendMessage {
            sender_name: String::from("sender name"),
            parent_id: msg.parent_id,
            content: msg.content,
        };
        Bot::start(test_func);
    }
}
