use crate::get_current_login;
use termimad::{print_inline, print_text};

use image::{DynamicImage, Pixel, Rgba, RgbaImage};

use reqwest::Client;
const BASE_URL: &str = "http://localhost:8081"; //9821
#[derive(Debug, serde::Deserialize)]
pub struct MessageResponce {
    pub payload: Vec<Message>,
    pub status: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Message {
    #[serde(default)]
    pub message_id: uuid::Uuid,
    pub sender_name: String,
    pub parent: Option<uuid::Uuid>,
    pub content: SendibleContent,
    #[serde(default)]
    pub sent_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum SendibleContent {
    Text(TextMessage),
    Img(ImgMessage),
    Title(TitleMessage),
}

impl SendibleContent {
    pub async fn show(&self) {
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
    pub fn get_content(&self) -> String {
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
pub struct SendMesage {
    pub sender_name: String,
    pub parent: Option<uuid::Uuid>,
    pub content: serde_json::Value,
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
        print_text(&fixed_input);
    }
}

#[derive(Debug, serde::Deserialize)]
struct TitleMessage {
    title: String,
}

impl MessageInterface for TitleMessage {
    async fn show(&self) {
        let raw = self.title.to_string();
        let fixed_input = raw.replace("\\n", "\n").replace("\\", "");
        print!("title: ");
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

        let img = download_img_from_db(url).await;

        let conf = viuer::Config {
            absolute_offset: false,
            ..Default::default()
        };
        println!("img: ");
        viuer::print(&img, &conf).expect("Image printing failed.");
    }
}
async fn download_img_from_db(url: &str) -> DynamicImage {
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

    let client = reqwest::Client::new();

    let response = client.get(&presigned_url).send().await.unwrap();
    let bytes = response.bytes().await.unwrap();

    image::load_from_memory(&bytes).expect("Failed to load image from memory")
}
