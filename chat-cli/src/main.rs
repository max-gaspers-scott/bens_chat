use reqwest::{self, get, post};
use tokio;
async fn get_health(id: String) -> Result<(), reqwest::Error> {
    println!("Hello, world!");
    // let url = format!("http://localhost:8081/messages?chat_id={}", id);

    let url = "http://localhost:8081/health";
    let body = reqwest::get(url).await?.text().await?;
    println!("resp is: {body:?}");
    Ok(())
}

// need to add this json as headers
// {'Content-Type': 'application/json',  Authorization: `Bearer ${token}`}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    get_health("123".to_string()).await;
    Ok(())
}

//
//  loging fn, to get token
//  put username and passowrd into json
//  add json headers (see comment above about applicaiton/json)
//    * probably dont need Bearer token for this
//  fit endpoint and get token
//

async fn login() -> Result<(), reqwest::Error> {
    let url = "http://localhst:8081/auth/login";
    let params = [("foo", "bar"), ("baz", "quux")];
    let client = reqwest::Client::new();
    let res = client.post(url).form(&params).send().await?;
    Ok(())
}
