use clap::error::ContextKind;
use core::slice;
use reqwest::{self, Client};
use serde::Deserialize;
use std::alloc::handle_alloc_error;
use std::collections::HashMap;
use std::fmt::format;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Condvar;
use uuid::Uuid;

// should be in env, but this will work for now
// const PORT: u32 = 8081;
const BASE_URL: &str = "http://localhost:9821";

#[derive(Debug)]
enum Stats {
    Login,
    Chats,        // viewing  creating and deleating chats
    Conversation, // viewing and sending messages
}

#[derive(Debug)]
enum Acction {
    Logout,
    Login,
    GotoChats,
    GotoConversation,
}

struct Window {
    state: Stats,
}
impl Window {
    fn new() -> Window {
        Window {
            state: Stats::Login,
        }
    }
    fn transition(&mut self, acction: Acction) {
        match (&self.state, acction) {
            (Stats::Login, Acction::Login) => self.state = Stats::Chats,
            (Stats::Chats, Acction::GotoConversation) => self.state = Stats::Conversation,
            (Stats::Conversation, Acction::GotoChats) => self.state = Stats::Chats,
            (_, Acction::Logout) => self.state = Stats::Login,
            (_, _) => self.state = Stats::Login,
        };
    }
    fn run(&mut self) {
        loop {
            // Match the state, execute the screen logic, and get the resulting action
            let action = match self.state {
                Stats::Login => self.handel_login(),
                Stats::Chats => self.handel_chats(),
                Stats::Conversation => self.handel_conversation(),
            };

            println!("acction: {:?}", action);

            // Transition the state based on what happened in the screen
            self.transition(action);
        }
    }
    fn handel_login(&mut self) -> Acction {
        println!("handel login");
        let mut name_buff = String::new();
        std::io::stdin()
            .read_line(&mut name_buff)
            .expect("could not read input buffer");

        Acction::Login
    }
    fn handel_chats(&mut self) -> Acction {
        println!("doing chats ");
        let mut name_buff = String::new();
        std::io::stdin()
            .read_line(&mut name_buff)
            .expect("could not read input buffer");

        Acction::GotoConversation
    }
    fn handel_conversation(&mut self) -> Acction {
        println!("handeling messages/convos ");
        let mut name_buff = String::new();
        std::io::stdin()
            .read_line(&mut name_buff)
            .expect("could not read input buffer");

        Acction::GotoChats
    }
}

#[derive(Debug, serde::Deserialize)]
struct MessageResponce {
    payload: Vec<Message>,
    status: String,
}

#[derive(Debug, serde::Deserialize)]
struct Message {
    message_id: uuid::Uuid,
    sender_id: uuid::Uuid,
    content: String,
    sent_at: String,
    minio_url: Option<String>,
    username: String,
}

async fn get_messages(
    login: &LoginPayload,
    chat_id: &Uuid,
) -> Result<Vec<Message>, reqwest::Error> {
    println!("chat-id id: {:?}", chat_id);
    let url = format!("{BASE_URL}/messages?chat_id={}", chat_id);

    let client = Client::new();

    let res = client
        .get(url)
        .bearer_auth(login.token.clone())
        .send()
        .await?;

    let message_responce: MessageResponce = res.json().await.unwrap();
    Ok(message_responce.payload)

    // let messages = res.text().await?;
    // println!("massages:\n {}", messages);
}

#[derive(Deserialize)]
struct ChatResponce {
    data: Vec<Chat>,
    status: String,
}

#[derive(Deserialize)]
struct Chat {
    chat_id: Uuid,
    chat_name: String,
    created_at: String,
    joined_at: String,
}

//TODO: need user id as query param
async fn get_chats(user_info: &LoginPayload) -> Result<ChatResponce, reqwest::Error> {
    println!("{}", user_info.user_id);
    println!("{}", user_info.token);
    let url = format!("{BASE_URL}/user-chats?user_id={}", user_info.user_id);

    let client = reqwest::Client::new();
    let res = client.get(url).bearer_auth(&user_info.token).send().await?;
    let chats: ChatResponce = res.json().await?;
    Ok(chats)
}

// need to add this json as headers
// {'Content-Type': 'application/json',  Authorization: `Bearer ${token}`}
//
#[derive(Eq, Hash, PartialEq)]
enum State {
    Chats,
    Messages,
}

async fn send_message(
    login: &LoginPayload,
    message: &str,
    chat_id: &Uuid,
) -> Result<(), reqwest::Error> {
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
    if selected_id
        == &Uuid::parse_str("00000000-0000-0000-0000-000000000000").expect("Error getting 0 uuid")
    {
        let mut name_buff = String::new();
        println!("what would you like the new chat to be called");
        std::io::stdin()
            .read_line(&mut name_buff)
            .expect("could not read input buffer");
        let mut recipiant_buffer = String::new();
        println!("who do you want to add to the chat");
        std::io::stdin()
            .read_line(&mut recipiant_buffer)
            .expect("could not read input buffer");

        // url parame nesisary ???
        let url = format!("BASE_URL/chat?chat_name={}", name_buff);
        let client = reqwest::Client::new();

        let payload = serde_json::json!({
            "chat_name": name_buff,
        });

        let send_res = client
            .post(url)
            .json(&payload)
            .bearer_auth(login.token.clone())
            .send()
            .await?;
        //
        // let url = format!("http://localhost:PORT/user_chats");
        // let client = reqwest::Client::new();
        //
        // let payload = serde_json::json!({
        //     "chat_name": name_buff,
        // });
        //
        // let send_res = client
        //     .post(url)
        //     .json(&payload)
        //     .bearer_auth(login.token.clone())
        //     .send()
        //     .await?;
    }
    let messages = get_messages(&login, selected_id).await.unwrap();
    print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    for m in messages {
        println!("{}: {}", m.username, m.content);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = Window::new();
    app.run();
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

#[derive(Deserialize)]
struct LoginResponse {
    payload: LoginPayload,
    status: String,
}

#[derive(Deserialize)]
struct LoginPayload {
    token: String,
    user_id: String,
}

async fn login() -> Result<LoginPayload, reqwest::Error> {
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
