use reqwest::{self, Client};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use uuid::Uuid;

async fn get_health() -> Result<(), reqwest::Error> {
    let url = "http://localhost:9821/health";
    let body = reqwest::get(url).await?.text().await?;
    println!("resp is: {body:?}");
    Ok(())
}

// async fn get_username(token: String, user_id: String) -> Resutl<String, reqwest::Error> {
//     let url = format!("http://localhost:9821/users");
// }

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
    let url = format!("http://localhost:9821/messages?chat_id={}", chat_id);

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
    let url = format!(
        "http://localhost:9821/user-chats?user_id={}",
        user_info.user_id
    );

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
    let url = format!("http://localhost:9821/messages?chat_id={}", chat_id);
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "chat_id": chat_id,
        "content": message,
    });

    let _ = client
        .post(url)
        .json(&payload)
        .bearer_auth(login.token.clone())
        .send()
        .await?;
    Ok(())
}

async fn show_messages(login: &LoginPayload, selected_id: &Uuid) -> Result<(), reqwest::Error> {
    let messages = get_messages(&login, selected_id).await.unwrap();
    print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    for m in messages {
        println!("{}: {}", m.username, m.content);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut fsa: HashMap<State, State> = HashMap::new();
    fsa.insert(State::Chats, State::Messages);
    let login = login().await?;

    let chats_raw = get_chats(&login);
    let chats = chats_raw.await.unwrap().data;
    let mut hashmap = HashMap::new();

    for c in chats {
        println!("chat: {}", c.chat_name);
        hashmap.insert(c.chat_name, c.chat_id);
    }

    let mut buff = String::new();
    println!("what chat do you want to see");

    std::io::stdin().read_line(&mut buff).unwrap();
    let input = buff.trim();
    let selected_id = hashmap.get(input).unwrap();

    show_messages(&login, selected_id).await.unwrap();

    loop {
        let mut message = String::new();
        std::io::stdin().read_line(&mut message)?;
        send_message(&login, &message, selected_id).await.unwrap();
        let input = buff.trim();
        let selected_id = hashmap.get(input).unwrap();
        if message.trim() == "/exit" {
            let chats_raw = get_chats(&login);
            let chats = chats_raw.await.unwrap().data;
            let mut hashmap = HashMap::new();
            for c in chats {
                println!("chat: {}", c.chat_name);
                hashmap.insert(c.chat_name, c.chat_id);
            }

            let mut buff = String::new();
            println!("what chat do you want to see");

            std::io::stdin().read_line(&mut buff)?;
            print!("{}[2J{}[1;1H", 27 as char, 27 as char);
        }
        show_messages(&login, selected_id).await.unwrap();
    }
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

    let url = "http://localhost:9821/auth/login";
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
