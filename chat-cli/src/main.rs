use image::{DynamicImage, Pixel, Rgba, RgbaImage};
use reqwest::{self, Client};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::future::Future;
use std::io::Write;
use std::time::Duration;
use termimad::print_text;
use uuid::Uuid;
use viuer::print;

// should be in env, but this will work for now
// const PORT: u32 = 8081;
const BASE_URL: &str = "http://localhost:9821"; //9821
// const BASE_URL: &str = "https://bens-chat.team-stingray.com";

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

#[derive(Deserialize, Debug)]
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
        let mut person = String::new();
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
        self.login = LoginInfo::Loggedin {
            info: user_login().await.unwrap(),
        };

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
                c.content.content["title"], c.content.content["text"]
            );

            let chat_name = if c.content.content["title"].is_null() {
                c.content.content["text"].as_str().unwrap()
            } else {
                c.content.content["title"]
                    .as_str()
                    .unwrap_or("title was not found")
            };

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
        println!("5");
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
            if message.trim() == "test" {
                println!("6");
                test_socket().await;
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

#[derive(Debug, serde::Deserialize)]
struct Message {
    #[serde(default)]
    message_id: uuid::Uuid,
    sender_name: String,
    parent: Option<uuid::Uuid>,
    #[serde(flatten)] // look into this
    content: SendibleContent,
    #[serde(default)]
    sent_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Deserialize)]
struct SendibleContent {
    content: serde_json::Value,
}

impl SendibleContent {
    fn show(&self) {
        let raw = self.content["text"].to_string();
        let fixed_input = raw.replace("\\n", "\n").replace("\\", "");
        print_text(&fixed_input);

        if !self.content["url"].is_null() {
            let file = self.content["url"].to_string();
            let conf = viuer::Config {
                ..Default::default()
            };
            // viuer::print_from_file("./moninoki.jpg", &conf).expect("Image printing failed.");
            println!("{file}")
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct SendMesage {
    sender_name: String,
    parent: Option<uuid::Uuid>,
    content: serde_json::Value,
}

impl Message {
    fn show(&self) {
        println!("id:{}\n{}: ", self.sender_name, self.message_id,);
        self.content.show();
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
        m.show();
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
    // let mut params = HashMap::new();
    // params.insert("username", name);
    // params.insert("password", password);
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

async fn test_socket() {
    println!("testing ");
    ()
}
