use reqwest::{self, Client, Proxy};
use serde::Deserialize;
use std::collections::hash_map;
use std::fs::OpenOptions;
use std::io::{LineWriter, Write};
use std::path::PathBuf;
use std::ptr::hash;
use std::{collections::HashMap, os::unix::process};
use std::{fs, io};
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

    let client = reqwest::Client::new();

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let login = login().await?;

    loop {
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer)?;
        let input = buffer.trim();
        println!("{}", buffer);

        match input {
            "login" => {}
            "chats" => {
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
                let input = buff.trim();
                let selected_id = hashmap.get(input).unwrap();
                let messages = get_messages(&login, selected_id).await.unwrap();
                print!("{}[2J{}[1;1H", 27 as char, 27 as char);
                for m in messages {
                    println!("{}: {}", m.sender_id, m.content);
                }
            }
            "health" => {
                let health = get_health().await;
                match health {
                    Ok(_) => println!("api healthey"),
                    Err(e) => print!("api error: {e}"),
                }
            }
            _ => println!("not an option"),
        }
    }
    get_health().await;
    // let user_info = login().await.unwrap();
    // let res = get_chats(&user_info).await.unwrap();
    // let chat_id = res.data[0].chat_id;
    //
    // let messages = get_messages(user_info.token.clone(), chat_id)
    //     .await
    //     .unwrap();
    // for m in messages {
    //     println!("{}: {}", m.username, m.content)
    // }

    Ok(())
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
    std::io::stdin().read_line(&mut name);

    println!("what is your password");
    let mut password = String::new();
    std::io::stdin().read_line(&mut password);
    let password = password.trim();
    let name = name.trim();

    let url = "http://localhost:9821/auth/login";
    let mut params = HashMap::new();
    params.insert("username", name);
    params.insert("password", password);
    println!("{:?}", params);
    let client = reqwest::Client::new();
    let res = match client.post(url).json(&params).send().await {
        Ok(r) => r,
        Err(e) => return Err(e),
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
