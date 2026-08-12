mod structs;
use crate::structs::*;
use clap::builder::Str;
use cool_cli_input::get_input;
use futures_util::{FutureExt, StreamExt};
use inquire::{
    Editor, Text,
    error::InquireResult,
    ui::{Color, RenderConfig, Styled},
};
use reqwest::Response;
use reqwest::{self, Client, Request};
use serde::Deserialize;
use serde_json::json;
use std::clone;
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::future::Future;
use std::io::Write;
use std::thread::AccessError;
use std::time::Duration;
use termimad::{print_inline, print_text};
use uuid::Uuid;
use viuer::print;

// should be in env, but this will work for now
// const PORT: u32 = 8081;
const BASE_URL: &str = "http://localhost:8081"; //9821
// const BASE_URL: &str = "https://bens-chat.team-stingray.com";

use std::sync::RwLock;

//TODO: a mutex / global var feels bad (code smell)
// and also requires setter and getter
static CURRENT_LOGIN: RwLock<Option<LoginPayload>> = RwLock::new(None);

fn set_current_login(payload: LoginPayload) {
    if let Ok(mut lock) = CURRENT_LOGIN.write() {
        *lock = Some(payload);
    }
}

fn get_current_login() -> Option<LoginPayload> {
    CURRENT_LOGIN.read().ok().and_then(|lock| lock.clone())
}

#[derive(Debug)]
enum Stats {
    Login,
    Signup,
    Chats, // viewing  creating and deleating chats
    NewChat,
    Conversation { chat_id: Uuid }, // viewing and sending messages
}

#[derive(Debug)]
enum Action {
    Logout,
    Login,
    SignUp,
    GotoChats,
    MakeChat,
    GotoConversation { chat_id: Uuid },
}
#[derive(serde::Serialize)]
struct ChatParticipant {
    chat_id: uuid::Uuid,
    user_name: String,
}

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

#[derive(Debug, serde::Deserialize)]
enum LoginInfo {
    Loggedin { info: LoginPayload },
    NotLoggedin,
}

struct Window {
    state: Stats,
    login: LoginInfo,
}
impl Window {
    fn new() -> Window {
        Window {
            state: Stats::Login,
            login: LoginInfo::NotLoggedin,
        }
    }
    fn transition(&mut self, action: Action) {
        match (&self.state, action) {
            (Stats::Login, Action::Login) => self.state = Stats::Chats,
            // (Stats::Login, _) => self.state = Stats::Login,
            (Stats::Login, Action::SignUp) => self.state = Stats::Signup,
            (Stats::Signup, Action::SignUp) => self.state = Stats::Login,

            (Stats::Chats, Action::GotoConversation { chat_id }) => {
                self.state = Stats::Conversation { chat_id }
            }
            (Stats::Conversation { chat_id: _ }, Action::GotoConversation { chat_id }) => {
                self.state = Stats::Conversation { chat_id }
            }

            (Stats::Chats, Action::MakeChat) => self.state = Stats::NewChat,
            (Stats::Conversation { chat_id }, Action::GotoChats) => self.state = Stats::Chats,
            (Stats::NewChat, Action::MakeChat) => self.state = Stats::Chats,
            (_, Action::Logout) => self.state = Stats::Login,
            _ => self.state = Stats::Login,
        };
    }
    async fn run(&mut self) {
        loop {
            let action = match &self.state {
                Stats::Login => self.handel_login().await,
                Stats::Signup => self.handel_signup().await,
                Stats::Chats => self.handel_chats().await,
                Stats::NewChat => self.handel_make_chat().await,
                Stats::Conversation { chat_id } => self.handel_conversation(*chat_id).await,
            };
            self.transition(action);
        }
    }

    async fn handel_signup(&mut self) -> Action {
        let name = inquire::Text::new("what is your name").prompt().unwrap();

        let phone_number = Some(get_input("what phone number"));

        let email = Some(get_input("what's your email"));

        let password_hash = inquire::Password::new("what password").prompt().unwrap();
        // hash it
        let user = User {
            phone_number,
            name,
            email,
            password_hash,
        };
        let url = format!("{BASE_URL}/users");
        let client = reqwest::Client::new();

        match client.post(url).json(&user).send().await {
            Ok(_) => Action::Login,
            Err(e) => {
                println!("error posting message: {e}");
                Action::SignUp
            }
        } // post to db
    }

    async fn handel_make_chat(&mut self) -> Action {
        let login_stuff = match &self.login {
            LoginInfo::Loggedin { info } => Some(info),
            LoginInfo::NotLoggedin => {
                println!("not logged in");
                None
            }
        }
        .unwrap();
        println!("make a chat");
        let title = get_input("your root message title: ");

        let title = title.trim();
        let content = serde_json::json!({
            "text": title,
        });
        if title == "/exit" {
            print!("{}[2J{}[1;1H", 27 as char, 27 as char);
            return Action::GotoChats;
        }
        let msg = SendMesage {
            sender_name: login_stuff.username.clone(),
            parent: None,
            content,
        };

        let msg_id = match send_message(login_stuff, &msg).await {
            Ok(PostMsgRes { data }) => data.message_id,
            Err(e) => {
                println!("error sendimg message: {e}");
                return Action::GotoChats;
            }
        };

        // add users to chat
        //
        let other_user_name = get_input("who do you want to chat with").trim().to_string();

        let other_user = ChatParticipant {
            chat_id: msg_id,
            user_name: other_user_name,
        };
        match post_userchat(login_stuff, &other_user).await {
            Ok(_) => println!("posted to user chats"),
            Err(e) => println!("error adding a user to the chat"),
        }

        Action::MakeChat
    }
    async fn handel_login(&mut self) -> Action {
        if get_input("make a new account? y/n").trim() == "y" {
            println!("new account");
            return Action::SignUp;
        }
        let info = user_login().await.unwrap();
        set_current_login(info.clone());
        self.login = LoginInfo::Loggedin { info };

        let uid = match &self.login {
            LoginInfo::Loggedin { info } => &info.username,
            LoginInfo::NotLoggedin => &String::from("no user id"),
        };

        Action::Login
    }
    async fn handel_chats(&mut self) -> Action {
        let login = match &self.login {
            LoginInfo::Loggedin { info } => info,
            LoginInfo::NotLoggedin => panic!(), //should never git to this point without login info
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
            println!("chat: {}", c.content.get_content());

            let chat_name = c.content.get_content();
            hashmap.insert(chat_name.to_string(), c.message_id);
        }

        loop {
            let chat_name = get_input("what chat do you want to see (or 'n' for new chat)");
            let input = chat_name.trim();
            if input == "n" {
                return Action::MakeChat;
            }

            if let Some(&selected_id) = hashmap.get(input) {
                return Action::GotoConversation {
                    chat_id: selected_id,
                };
            } else {
                println!("Chat '{}' not found. Please try again.", input);
            }
        }
    }
    async fn handel_conversation(&mut self, chat_id: Uuid) -> Action {
        let login_stuff = match &self.login {
            LoginInfo::Loggedin { info } => Some(info),
            LoginInfo::NotLoggedin => {
                println!("not logged in");
                None
            }
        }
        .unwrap();
        loop {
            get_and_show_msg(&login_stuff, &chat_id).await;
            println!("------------------");
            let message = get_input("your message: ");

            let content = serde_json::json!({
                "text": message.trim(),
            });
            let msg = SendMesage {
                sender_name: login_stuff.username.clone(),
                parent: Some(chat_id),
                content,
            };

            if message.trim() == "/update" {
                get_and_show_msg(&login_stuff, &chat_id).await;
                continue;
            }

            if message.trim() == "/exit" {
                print!("{}[2J{}[1;1H", 27 as char, 27 as char);

                return Action::GotoChats;
            }
            if message.trim() == "/subchat" {
                let mut buff = String::new();
                println!("what chat do you want to see");

                std::io::stdin().read_line(&mut buff).unwrap();
                let input = buff.trim();
                return Action::GotoConversation {
                    chat_id: Uuid::parse_str(input).unwrap(),
                };
            }
            match send_message(login_stuff, &msg).await {
                Ok(_) => {}
                Err(e) => print!("error sendimg message: {e}"),
            }
        }
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
    let text = res.text().await?;
    println!("DEBUG get_messages raw body: {}", text);
    let message_responce: MessageResponce = serde_json::from_str(&text)
        .map_err(|e| {
            println!("JSON parsing error in get_messages: {}", e);
            panic!("Failed to parse messages JSON");
        })
        .unwrap();

    Ok(message_responce.payload)
}

#[derive(Deserialize)]
struct ChatResponce {
    payload: Vec<Message>,
    status: String,
}

async fn get_chats(user_info: &LoginPayload) -> Result<ChatResponce, reqwest::Error> {
    let url = format!("{BASE_URL}/user-chats?username={}", user_info.username);

    let client = reqwest::Client::new();
    let res = client.get(url).bearer_auth(&user_info.token).send().await?;
    let text = res.text().await?;
    let chats: ChatResponce = serde_json::from_str(&text)
        .map_err(|e| {
            println!("JSON parsing error in get_chats: {}", e);
            panic!("Failed to parse chats JSON");
        })
        .unwrap();

    Ok(chats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_chats() {
        let raw_json = r#"{"payload":[{"content":{"title":"chat1"},"message_id":"f79f1427-436d-47e2-8c47-aed6ff2bf09d","sender_name":"a","sent_at":"2026-06-23T17:24:23.562447Z"},{"content":{"text":"hi\n"},"message_id":"64f792b3-dc94-424f-be7e-b5cc67ee8541","sender_name":"a","sent_at":"2026-06-23T17:24:31.023501Z"},{"content":{"title":"chat2"},"message_id":"c8ed85b9-2c60-4c87-ba62-21a0212c5433","sender_name":"a","sent_at":"2026-06-23T17:41:58.152064Z"},{"content":{"text":"chat1"},"message_id":"65e3936f-261f-4839-ab06-85244e618eef","sender_name":"a","sent_at":"2026-06-23T19:24:10.642075Z"},{"content":{"text":"chat2"},"message_id":"85d12ced-eac4-4c7b-b924-3e0afafac189","sender_name":"a","sent_at":"2026-06-27T17:22:17.348149Z"},{"content":{"text":"chat3"},"message_id":"80795351-3872-4d3b-98ec-bb3d0a8a350b","sender_name":"a","sent_at":"2026-07-07T19:21:32.874907Z"},{"content":{"text":"oneOff"},"message_id":"3fa5477f-0b03-4138-ace6-3b2093160447","sender_name":"a","sent_at":"2026-07-07T22:52:29.002109Z"},{"content":{"text":"oneOff"},"message_id":"34816fb5-ba05-428b-a83d-e8de04775099","sender_name":"a","sent_at":"2026-07-08T18:22:17.177489Z"},{"content":{"text":"a_b"},"message_id":"130b18ea-4d97-4b45-9e01-b16390b9f158","sender_name":"b","sent_at":"2026-07-09T23:05:56.530122Z"},{"content":{"text":"a_b_test"},"message_id":"f0a65207-39a6-498f-a432-52a3b44f8622","sender_name":"a","sent_at":"2026-07-09T23:06:51.625501Z"},{"content":{"title":"a & b"},"message_id":"30a7bccb-cb81-4dbb-8f0f-4558b1b51763","sender_name":"a","sent_at":"2026-07-16T13:03:45.536795Z"},{"content":{"text":"canata"},"message_id":"bd97da05-07f8-45f3-bc01-226fdb16b615","sender_name":"a","sent_at":"2026-07-18T20:07:13.754509Z"},{"content":{"text":"iter"},"message_id":"8f563e28-5ef2-4025-8caa-ecf362ce077b","sender_name":"a","sent_at":"2026-07-19T16:44:00.876965Z"}],"status":"success"}"#;
        let chats: ChatResponce = serde_json::from_str(raw_json).unwrap();
        assert_eq!(chats.status, "success");
        assert_eq!(chats.payload.len(), 13);
    }
}

#[derive(Deserialize, Debug)]
struct PostMsgData {
    message_id: uuid::Uuid,
}
#[derive(Deserialize, Debug)]
struct PostMsgRes {
    data: PostMsgData,
}
async fn send_message(
    login: &LoginPayload,
    message: &SendMesage,
) -> Result<PostMsgRes, reqwest::Error> {
    let url = format!("{BASE_URL}/messages");
    let client = reqwest::Client::new();

    let response = client
        .post(url)
        .json(message)
        .bearer_auth(login.token.clone())
        .send()
        .await?;

    let parsed_res = response.json::<PostMsgRes>().await?;
    Ok(parsed_res)
}

async fn post_userchat(
    login: &LoginPayload,
    chat_participant: &ChatParticipant,
) -> Result<(), reqwest::Error> {
    let url = format!("{BASE_URL}/user-chats");
    let client = reqwest::Client::new();

    match client
        .post(url)
        .json(chat_participant)
        .bearer_auth(login.token.clone())
        .send()
        .await
    {
        Ok(_) => {}
        Err(e) => println!("error posting message: {e}"),
    }

    Ok(())
}

async fn show_messages(messages: &[Message]) -> Result<(), reqwest::Error> {
    print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    for m in messages {
        if m.sender_name == get_current_login().unwrap().username {
            print!("you: ");
        } else {
            print!("{}: ", m.sender_name);
        }
        m.content.show().await;
    }
    Ok(())
}

async fn get_and_show_msg(login_stuff: &LoginPayload, chat_id: &Uuid) {
    let messages = get_messages(&login_stuff, &chat_id).await.unwrap();
    show_messages(&messages).await.unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = Window::new();

    app.run().await;
    Ok(())
}

async fn user_login() -> Result<LoginPayload, reqwest::Error> {
    loop {
        let name = get_input("what is your name");

        let password = inquire::Password::new("what is your password")
            .prompt()
            .unwrap();
        let password = password.trim();
        let name = name.trim();

        let url = format!("{BASE_URL}/auth/login");
        let payload = serde_json::json!({
            "username": name,
            "password": password,
        });

        let client = reqwest::Client::new();

        let res = match client.post(url).json(&payload).send().await {
            Ok(res) => res,
            Err(e) => {
                println!("Network or request error: {e}. Please try again.");
                continue;
            }
        };

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            println!(
                "Login failed (status {}): {}. Please try again.",
                status, body
            );
            continue;
        }

        let text = match res.text().await {
            Ok(text) => text,
            Err(e) => {
                println!("Failed to read response: {e}. Please try again.");
                continue;
            }
        };

        let data: LoginResponse = match serde_json::from_str(&text) {
            Ok(data) => data,
            Err(e) => {
                println!("Could not parse login response ({e}). Please try again.");
                continue;
            }
        };

        let user_info = data.payload;

        print!("{}[2J{}[1;1H", 27 as char, 27 as char);
        return Ok(user_info);
    }
}

pub fn write_file(path: &std::path::Path, text: &str) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().create(true).write(true).open(path)?;
    file.write_all(text.as_bytes())?;
    file.flush()?;
    Ok(())
}
