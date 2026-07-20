use clap::builder::Str;
use futures_util::StreamExt;
use image::{DynamicImage, Pixel, Rgba, RgbaImage};
use rand::RngExt;
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
use std::time::Duration;
use termimad::minimad::Text;
use termimad::{print_inline, print_text};
use uuid::Uuid;
use viuer::print;

// should be in env, but this will work for now
// const PORT: u32 = 8081;
const BASE_URL: &str = "http://localhost:9821"; //9821
// const BASE_URL: &str = "https://bens-chat.team-stingray.com";

use std::sync::RwLock;

//TODO: i dont like this code. a mutex / global var feels bad
// and also requiers setter and getter
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
    Chats, // viewing  creating and deleating chats
    NewChat,
    Conversation { chat_id: Uuid }, // viewing and sending messages
}

#[derive(Debug)]
enum Action {
    Logout,
    Login,
    GotoChats,
    MakeChat,
    GotoConversation { chat_id: Uuid },
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
    fn transition(&mut self, Action: Action) {
        match (&self.state, Action) {
            (Stats::Login, Action::Login) => self.state = Stats::Chats,
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
                Stats::Chats => self.handel_chats().await,
                Stats::NewChat => self.handel_make_chat().await,
                Stats::Conversation { chat_id } => self.handel_conversation(*chat_id).await,
            };
            self.transition(action);
        }
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
        println!("your root message title: ");
        let mut title = String::new();
        match std::io::stdin().read_line(&mut title) {
            Ok(_) => {}
            Err(e) => println!("an error reading from buffer: {e}"),
        }
        let title = title.trim();
        let content = serde_json::json!({
            "text": title,
        });
        let msg = SendMesage {
            sender_name: login_stuff.username.clone(),
            parent: None,
            content,
        };
        //TODO: make new stat for adding people to chat
        //should let anyone in a chant add a new user???
        // while person != "/q" {
        //     println!("who do you want to add to the chat. /q to exit");
        //     match std::io::stdin().read_line(&mut person) {
        //         Ok(_) => {}
        //         Err(e) => println!("error reading person name"),
        //     }
        //     let person = person.trim();
        // }

        match send_message(login_stuff, &msg).await {
            Ok(_) => {}
            Err(e) => println!("error sendimg message: {e}"),
        }

        if title == "/exit" {
            print!("{}[2J{}[1;1H", 27 as char, 27 as char);
            return Action::GotoChats;
        }
        Action::MakeChat
    }
    async fn handel_login(&mut self) -> Action {
        let info = user_login().await.unwrap();
        set_current_login(info.clone());
        self.login = LoginInfo::Loggedin { info };

        let uid = match &self.login {
            LoginInfo::Loggedin { info } => &info.username,
            LoginInfo::NotLoggedin => &String::from("no user id"),
        };
        println!("{uid}");

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
            println!(
                "chat: {},{}",
                c.content.get_content(),
                c.content.get_content()
            );

            // let content = c.content.get_content();
            // let chat_name = if content["title"].is_null() {
            //     content["text"].as_str().unwrap()
            // } else {
            //     content["title"].as_str().unwrap_or("title was not found")
            // };

            let chat_name = c.content.get_content();
            hashmap.insert(chat_name.to_string(), c.message_id);
        }

        let mut buff = String::new();
        println!("what chat do you want to see");

        std::io::stdin().read_line(&mut buff).unwrap();
        let input = buff.trim();
        if input == "n" {
            return Action::MakeChat;
        }

        println!("your input: {input}");
        let selected_id = hashmap.get(input).unwrap();

        Action::GotoConversation {
            chat_id: *selected_id,
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
            println!("your message: ");
            let mut message = String::new();
            std::io::stdin().read_line(&mut message);
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

#[derive(Debug, serde::Deserialize)]
struct MessageResponce {
    payload: Vec<Message>,
    status: String,
}

// #{buffers} get_messages puts the json result into dmessageResponce, wich has an araray of Messages, and a dmessages has a SendibleContent. SendibleContent is an emun of to variants that have json values as there fields. the compiler cant get the json filed into the enum

#[derive(Debug, serde::Deserialize)]
struct Message {
    #[serde(default)]
    message_id: uuid::Uuid,
    sender_name: String,
    parent: Option<uuid::Uuid>,
    content: SendibleContent,
    #[serde(default)]
    sent_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum SendibleContent {
    Text(TextMessage),
    Img(ImgMessage),
    Title(TitleMessage),
}

impl SendibleContent {
    async fn show(&self) {
        match self {
            Self::Text(t) => {
                let _ = t.show().await;
            }
            Self::Img(i) => {
                let _ = i.show().await;
            }
            Self::Title(t) => {
                let _ = t.show().await;
            }
        }
    }
    fn get_content(&self) -> String {
        match self {
            Self::Text(t) => t.text.clone(),
            Self::Img(i) => i.url.clone(),
            Self::Title(t) => t.title.clone(),
        }
    }
}

#[derive(serde::Deserialize)]
struct Img {
    url: String,
}

#[derive(Debug, serde::Serialize)]
struct SendMesage {
    sender_name: String,
    parent: Option<uuid::Uuid>,
    content: serde_json::Value,
}

trait MessageInterface {
    async fn show(&self);
}

#[derive(Debug, serde::Deserialize)]
struct TextMessage {
    text: String,
}

impl MessageInterface for TextMessage {
    async fn show(&self) {
        let raw = self.text.to_string();
        let fixed_input = raw.replace("\\n", "\n").replace("\\", "");
        print!("text: ");
        print_text(&fixed_input);
    }
}

#[derive(Debug, serde::Deserialize)]
struct ImgMessage {
    url: String,
}

impl MessageInterface for ImgMessage {
    async fn show(&self) {
        let path = self.url.clone();
        let url = &format!("{BASE_URL}/minio-fetch?object_key={}", path);

        let output_filepath = download_img_from_db(url).await;

        let conf = viuer::Config {
            absolute_offset: false,
            ..Default::default()
        };
        println!("img: ");
        viuer::print_from_file(output_filepath, &conf).expect("Image printing failed.");

        //TODO: download the img to disk to dispaly
        // maybe can you Request
        // or see if there is an media screaming crate
        //
    }
}
async fn download_img_from_db(url: &str) -> String {
    let login = get_current_login().expect("No current login payload found");
    let client = Client::new();

    let res = client
        .get(url)
        .bearer_auth(login.token.clone())
        .send()
        .await
        .unwrap();
    let res: Img = res.json().await.map_err(|e| println!("{e}")).unwrap();

    let presigned_url = res.url;
    // The path where you want to save the downloaded file
    let rand_id = rand::rng().random_range(1000..=9999);
    let output_filepath = format!("downloaded_image_{}.png", rand_id);

    let client = reqwest::Client::new();

    // 3. Make the GET request
    let response = client.get(&presigned_url).send().await.unwrap();

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&output_filepath)
        .unwrap();

    // let mut file = File::create(presigned_url).unwrap();

    // 6. Stream the response body and write it to the file
    let mut stream = response.bytes_stream();
    let mut downloaded_bytes = 0;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.unwrap(); // Handle potential errors in receiving a chunk
        file.write_all(&chunk).unwrap(); // Write the chunk to the file
        downloaded_bytes += chunk.len();
    }

    file.flush().unwrap();
    output_filepath
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

//TODO: need user id as query param
async fn get_chats(user_info: &LoginPayload) -> Result<ChatResponce, reqwest::Error> {
    println!("{}", user_info.username);
    println!("{}", user_info.token);
    let url = format!("{BASE_URL}/user-chats?username={}", user_info.username);

    let client = reqwest::Client::new();
    let res = client.get(url).bearer_auth(&user_info.token).send().await?;
    let text = res.text().await?;
    println!("DEBUG get_chats raw body: {}", text);
    let chats: ChatResponce = serde_json::from_str(&text)
        .map_err(|e| {
            println!("JSON parsing error in get_chats: {}", e);
            panic!("Failed to parse chats JSON");
        })
        .unwrap();
    println!("get chats status: {:?}", chats.status);

    Ok(chats)
}

async fn send_message(login: &LoginPayload, message: &SendMesage) -> Result<(), reqwest::Error> {
    println!("running send message");
    let url = format!("{BASE_URL}/messages");
    let client = reqwest::Client::new();

    match client
        .post(url)
        .json(message)
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
    let payload = serde_json::json!({
        "username": name,
        "password": password,
    });

    let client = reqwest::Client::new();

    let res = client.post(url).json(&payload).send().await?;

    //TODO: data may come back as {error: "messages"}
    //whitch can not be turned into a LoginPayload, and will error.

    let data: LoginResponse = match res.json().await {
        Ok(res) => res,
        Err(e) => panic!("could not get api res into LoginRes: {e}"),
    };
    let user_info = data.payload;

    let path = std::path::Path::new("./token.txt");
    match write_file(path, &user_info.token) {
        Ok(_) => {}
        Err(e) => println!("write failed error: {}", e),
    }

    print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    Ok(user_info)
}

pub fn write_file(path: &std::path::Path, text: &str) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().create(true).write(true).open(path)?;
    file.write_all(text.as_bytes())?;
    file.flush()?;
    Ok(())
}
