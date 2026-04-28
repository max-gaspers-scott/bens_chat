use reqwest::{self, Client};
use serde::Deserialize;
use std::{collections::HashMap, os::unix::process};
use uuid::Uuid;
async fn get_health() -> Result<(), reqwest::Error> {
    let url = "http://localhost:8081/health";
    let body = reqwest::get(url).await?.text().await?;
    println!("resp is: {body:?}");
    Ok(())
}

async fn get_messages(token: String) -> Result<(), reqwest::Error> {
    let url = "http://localhost:8081/messages?chat_id=0";
    let mut headers = HashMap::new();
    let auth = format!("Bearer {}", token);

    headers.insert("Authorization", auth);
    let client = reqwest::Client::new();

    let res = client.get(url).header("Bearer", token).send().await?;
    println!("{:?}", res);
    Ok(())
}

#[derive(Deserialize)]
struct ChatResponce {
    data: Vec<Chat>,
    succsess: String,
}

#[derive(Deserialize)]
struct Chat {
    chat_id: Uuid,
    chat_name: String,
    created_at: String,
    joind_at: String,
}

//TODO: need user id as query param
async fn get_chats(user_info: LoginPayload) -> Result<(), reqwest::Error> {
    println!("{}", user_info.user_id);
    println!("{}", user_info.token);
    let url = format!(
        "http://localhost:8081/user-chats?user_id={}",
        user_info.user_id
    );

    let client = reqwest::Client::new();
    // let res = client.get(url).bearer_auth(user_info.token).send().await?;
    let res = client.get(url).bearer_auth(user_info.token).send().await?;
    let chats: ChatResponce = res.json().await?;
    println!("{:?}", chats.data[0].chat_name);
    Ok(())
}

// need to add this json as headers
// {'Content-Type': 'application/json',  Authorization: `Bearer ${token}`}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    get_health().await;
    let user_info = login().await.unwrap();
    let res = get_chats(user_info).await;

    match res {
        Ok(chats) => println!("got chats {:?}", chats),
        Err(e) => println!("error getting chats: {}", e),
    }

    Ok(())
}

//
//  loging fn, to get token
//  put username and passowrd into json
//  add json headers (see comment above about applicaiton/json)
//    * probably dont need Bearer token for this
//  fit endpoint and get token
//

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
    println!("running loging funk");
    let url = "http://localhost:8081/auth/login";
    // let params = [("username", "bar"), ("password", "quux")];
    let mut params = HashMap::new();
    params.insert("username", "test11");
    params.insert("password", "aaa");
    let client = reqwest::Client::new();
    println!("made it past client::new");
    let res = client.post(url).json(&params).send().await?;
    let data: LoginResponse = res.json().await?;
    let user_info = data.payload;
    Ok(user_info)
}
