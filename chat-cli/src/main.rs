use chrono;
use clap::error::ContextKind;
use core::slice;
use reqwest::{self, Client};
use serde::Deserialize;
use serde_json;
use std::alloc::handle_alloc_error;
use std::collections::HashMap;
use std::fmt::format;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Condvar;
use uuid::Uuid;

// should be in env, but this will work for now
// const PORT: u32 = 8081;
const BASE_URL: &str = "http://localhost:8081";

#[derive(Debug)]
enum Stats {
    Login,
    Chats,                          // viewing  creating and deleating chats
    Conversation { chat_id: Uuid }, // viewing and sending messages
}

#[derive(Debug)]
enum Acction {
    Logout,
    Login,
    GotoChats,
    GotoConversation { chat_id: Uuid },
}

#[derive(Deserialize)]
struct LoginResponse {
    payload: LoginPayload,
    status: String,
}

#[derive(Deserialize, Debug)]
struct LoginPayload {
    token: String,
    username: String,
}
enum LoginInfo {
    Logedin { info: LoginPayload },
    NotLogedin,
}

struct Window {
    state: Stats,
    login: LoginInfo,
}
impl Window {
    fn new() -> Window {
        Window {
            state: Stats::Login,
            login: LoginInfo::NotLogedin,
        }
    }
    fn transition(&mut self, acction: Acction) {
        match (&self.state, acction) {
            (Stats::Login, Acction::Login) => self.state = Stats::Chats,
            (Stats::Chats, Acction::GotoConversation { chat_id }) => {
                self.state = Stats::Conversation { chat_id: chat_id }
            }
            (Stats::Conversation { chat_id }, Acction::GotoChats) => self.state = Stats::Chats,
            (_, Acction::Logout) => self.state = Stats::Login,
            _ => self.state = Stats::Login,
        };
    }
    async fn run(&mut self) {
        loop {
            // Match the state, execute the screen logic, and get the resulting action
            let action = match &self.state {
                Stats::Login => self.handel_login().await,
                Stats::Chats => self.handel_chats().await,
                Stats::Conversation { chat_id } => self.handel_conversation(*chat_id).await,
            };

            println!("acction: {:?}", action);

            // Transition the state based on what happened in the screen
            self.transition(action);
        }
    }
    async fn handel_login(&mut self) -> Acction {
        self.login = LoginInfo::Logedin {
            info: user_login().await.unwrap(),
        };

        let uid = match &self.login {
            LoginInfo::Logedin { info } => &info.username,
            LoginInfo::NotLogedin => &String::from("no user id"),
        };
        println!("{uid}");

        Acction::Login
    }
    async fn handel_chats(&mut self) -> Acction {
        let login = match &self.login {
            LoginInfo::Logedin { info } => info,
            LoginInfo::NotLogedin => panic!(), //should never git to this point without login info
                                               //because we have to go thought login to get here
        };
        let chats_raw = get_chats(login);
        let chats = match chats_raw.await {
            Ok(chats) => chats,
            Err(e) => {
                println!("error with the api {:?}", e);
                panic!();
            }
        }
        .payload;
        let mut hashmap = HashMap::new();

        for c in chats {
            println!("chat: {}", c.content);

            let chat_name = c.content["name"]
                .as_str()
                .ok_or_else(|| format!("faild to get root messge name: {}", c.message_id))
                .unwrap();

            hashmap.insert(chat_name.to_string(), c.message_id);
        }

        let mut buff = String::new();
        println!("what chat do you want to see");

        std::io::stdin().read_line(&mut buff).unwrap();
        let input = buff.trim();
        let selected_id = hashmap.get(input).unwrap();

        Acction::GotoConversation {
            chat_id: *selected_id,
        }
    }
    async fn handel_conversation(&mut self, chat_id: Uuid) -> Acction {
        let login_stuff = match &self.login {
            LoginInfo::Logedin { info } => Some(info),
            LoginInfo::NotLogedin => {
                println!("not loged in");
                None
            }
        }
        .unwrap();
        loop {
            show_messages(&login_stuff, &chat_id).await.unwrap();
            println!("------------------");
            println!("your message: ");
            let mut message = String::new();
            std::io::stdin().read_line(&mut message);
            match send_message(login_stuff, &message, &chat_id).await {
                Ok(_) => {}
                Err(e) => print!("error sendimg message: {e}"),
            }
            if message.trim() == "/update" {
                show_messages(&login_stuff, &chat_id).await.unwrap();
            }
            if message.trim() == "/exit" {
                print!("{}[2J{}[1;1H", 27 as char, 27 as char);

                return Acction::GotoChats;
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
enum MesgTypes {
    Payload(TextMessage), // should be called Text
    Media(MediaMessage),
}

impl Showable for MesgTypes {
    fn show(&self) {
        match self {
            MesgTypes::Payload(msg) => msg.show(),
            MesgTypes::Media(msg) => msg.show(),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct MessageResponce {
    payload: Vec<Message>,
    status: String,
}

pub trait Showable {
    fn show(&self);
}

#[derive(Debug, serde::Deserialize)]
enum ChatMessage {
    Text(TextMessage),
    Media(MediaMessage),
}
impl Showable for ChatMessage {
    fn show(&self) {
        match self {
            ChatMessage::Text(msg) => msg.show(),
            ChatMessage::Media(media) => media.show(),
        }
    }
}
#[derive(Debug, serde::Deserialize)]
struct TextMessage {
    message_id: uuid::Uuid,
    sender_id: uuid::Uuid,
    content: String,
    sent_at: String,
    username: String,
}

#[derive(Debug, serde::Deserialize)]
struct Message {
    #[serde(default)]
    message_id: uuid::Uuid,
    sender_name: String,
    parent: Option<uuid::Uuid>,
    content: serde_json::Value,
    #[serde(default)]
    sent_at: chrono::DateTime<chrono::Utc>,
}

impl Message {
    fn show(&self) {
        println!("{}: {}", self.sender_name, self.content);
    }
}

impl TextMessage {
    fn from_message(msg: ChatMessage) -> Option<TextMessage> {
        match msg {
            ChatMessage::Text(test_msg) => Some(test_msg),
            ChatMessage::Media(_) => None,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct MediaMessage {
    message_id: uuid::Uuid,
    sender_id: uuid::Uuid,
    content: String,
    sent_at: String,
    minio_url: String,
    username: String,
}

impl Showable for MediaMessage {
    fn show(&self) {
        println!("######### show an image ########")
    }
}

impl Showable for TextMessage {
    fn show(&self) {
        println!("username: {}: {}", self.username, self.content);
    }
}

async fn get_messages(
    login: &LoginPayload,
    chat_id: &Uuid,
) -> Result<Vec<Message>, reqwest::Error> {
    println!("chat-id id: {:?}", chat_id);
    let url = format!("{BASE_URL}/messages?parent={}", chat_id);

    let client = Client::new();

    let res = client
        .get(url)
        .bearer_auth(login.token.clone())
        .send()
        .await?;
    // let raw_text = res.text().await?;
    // println!("RAW JSON FROM BACKEND:\n{}", raw_text);
    //
    // let message_response: MessageResponce = serde_json::from_str(&raw_text)
    //     .map_err(|e| {
    //         println!("SERDE ERROR DETAILED: {:?}", e);
    //         e
    //     })
    //     .unwrap(); // Temporarily leave unwrap just to see the print statement above
    let message_responce: MessageResponce = res.json().await.map_err(|e| println!("{e}")).unwrap();

    // .map(|v| {
    // let data = v.payload;
    // data
    // });
    Ok(message_responce.payload)

    // let messages = res.text().await?;
    // println!("massages:\n {}", messages);
}

#[derive(Deserialize)]
struct ChatResponce {
    payload: Vec<Message>,
    status: String,
}

//TODO: need user id as query param
async fn get_chats(user_info: &LoginPayload) -> Result<ChatResponce, reqwest::Error> {
    println!("{}", user_info.username);
    println!("{}", user_info.token);
    let url = format!("{BASE_URL}/user-chats?username={}", user_info.username);

    let client = reqwest::Client::new();
    let res = client.get(url).bearer_auth(&user_info.token).send().await?;
    let chats: ChatResponce = res.json().await?;
    println!("{:?}", chats.status);

    Ok(chats)
}

async fn send_message(
    login: &LoginPayload,
    message: &str,
    chat_id: &Uuid,
) -> Result<(), reqwest::Error> {
    println!("running send message");
    let url = format!("{BASE_URL}/messages?chat_id={}", chat_id);
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "chat_id": chat_id,
        "content": message,
    });

    let send_res = client
        .post(url)
        .json(&payload)
        .bearer_auth(login.token.clone())
        .send()
        .await?;
    Ok(())
}

async fn show_messages(login: &LoginPayload, selected_id: &Uuid) -> Result<(), reqwest::Error> {
    // TODO: be less stupid about how to tell wehn the user is making a new chat
    // maybe time to go back to fsa for movign between create_chat, chat, and pick_chat states
    // if selected_id
    //     == &Uuid::parse_str("00000000-0000-0000-0000-000000000000").expect("Error getting 0 uuid")
    // {
    //     let mut name_buff = String::new();
    //     println!("what would you like the new chat to be called");
    //     std::io::stdin()
    //         .read_line(&mut name_buff)
    //         .expect("could not read input buffer");
    //     let mut recipiant_buffer = String::new();
    //     println!("who do you want to add to the chat");
    //     std::io::stdin()
    //         .read_line(&mut recipiant_buffer)
    //         .expect("could not read input buffer");
    //
    //     // url parame nesisary ???
    //     let url = format!("BASE_URL/chat?chat_name={}", name_buff);
    //     let client = reqwest::Client::new();
    //
    //     let payload = serde_json::json!({
    //         "chat_name": name_buff,
    //     });
    //
    //     let send_res = client
    //         .post(url)
    //         .json(&payload)
    //         .bearer_auth(login.token.clone())
    //         .send()
    //         .await?;
    // }

    let messages = get_messages(&login, selected_id).await.unwrap();
    print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    for m in messages {
        m.show();
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = Window::new();
    app.run().await;
    Ok(())
    //
    // let login = login().await?;
    //
    // let chats_raw = get_chats(&login);
    // let chats = chats_raw.await.unwrap().data;
    // let mut hashmap = HashMap::new();
    //
    // for c in chats {
    //     println!("chat: {}", c.chat_name);
    //     hashmap.insert(c.chat_name, c.chat_id);
    // }
    //
    // let mut buff = String::new();
    // println!("what chat do you want to see");
    //
    // std::io::stdin().read_line(&mut buff).unwrap();
    // let input = buff.trim();
    // let selected_id = hashmap.get(input).unwrap();
    //
    // show_messages(&login, selected_id).await.unwrap();
    //
    // loop {
    //     let mut message = String::new();
    //     std::io::stdin().read_line(&mut message)?;
    //     match send_message(&login, &message, selected_id).await {
    //         Ok(_) => {}
    //         Err(e) => print!("error sendimg message: {e}"),
    //     }
    //     let input = buff.trim();
    //     let mut selected_id = hashmap.get(input).unwrap();
    //     if message.trim() == "/update" {
    //         show_messages(&login, selected_id).await.unwrap();
    //     }
    //     if message.trim() == "/exit" {
    //         let chats_raw = get_chats(&login);
    //         let chats = chats_raw.await.unwrap().data;
    //         let mut hashmap = HashMap::new();
    //         for c in chats {
    //             println!("chat: {}", c.chat_name);
    //             hashmap.insert(c.chat_name, c.chat_id);
    //         }
    //         // 74f3e359-97b1-4db0-82bc-fc83fd79471d
    //         // 00000000-0000-0000-0000-000000000000
    //         hashmap.insert(
    //             "new".to_string(),
    //             Uuid::parse_str("00000000-0000-0000-0000-000000000000")
    //                 .expect("Error getting 0 uuid"),
    //         );
    //
    //         let mut buff = String::new();
    //         println!("what chat do you want to see");
    //
    //         std::io::stdin().read_line(&mut buff)?;
    //         // selected_id = hashmap.get(&buff).unwrap();
    //
    //         print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    //     }
    //     show_messages(&login, selected_id).await.unwrap();
    // }
}

async fn user_login() -> Result<LoginPayload, reqwest::Error> {
    println!("what is your name");
    let mut name = String::new();
    match std::io::stdin().read_line(&mut name) {
        Ok(_) => {}
        Err(e) => {
            println!("error getting name: {e}");
        }
    }

    println!("what is your password");
    let mut password = String::new();
    match std::io::stdin().read_line(&mut password) {
        Ok(_) => {}
        Err(e) => {
            print!("error getting password: {e}");
        }
    }
    let password = password.trim();
    let name = name.trim();

    let url = format!("{BASE_URL}/auth/login");
    let mut params = HashMap::new();
    params.insert("username", name);
    params.insert("password", password);
    let client = reqwest::Client::new();
    let res = match client.post(url).json(&params).send().await {
        Ok(r) => r,
        Err(e) => {
            print!("error loging in");
            return Err(e);
        }
    };

    //TODO: data may come back as {error: "messages"}
    //wich can not be turned into a LoginPayload, and will error.

    let data: LoginResponse = res.json().await?;
    let user_info = data.payload;

    let path = std::path::Path::new("./token.txt");
    match write_file(path, &user_info.token) {
        Ok(_) => {}
        Err(e) => println!("write fiel error: {}", e),
    }

    print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    println!("loged in as: {}", params.get("username").unwrap());
    Ok(user_info)
}

// save token
// cli
// login
// see chats
// go into a chat and see messages
// send messages
//
//
pub fn write_file(path: &std::path::Path, text: &str) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().create(true).write(true).open(path)?;
    file.write_all(text.as_bytes())?;
    file.flush()?;
    Ok(())
}
