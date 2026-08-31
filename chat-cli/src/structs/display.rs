use bens_chat_shared::{Chip, Connect4, ImgMessage, SendableContent, TextMessage, TitleMessage};
use crate::get_current_login;
use image::DynamicImage;
use reqwest::Client;
use termimad::print_text;

const BASE_URL: &str = "https://bens-chat.team-stingray.com";

/// CLI-only trait for rendering a message to the terminal.
pub trait Showable {
    async fn show(&self);
}

impl Showable for SendableContent {
    async fn show(&self) {
        match self {
            Self::Text(t) => t.show().await,
            Self::Img(i) => i.show().await,
            Self::Title(t) => t.show().await,
            Self::Con4(t) => t.show().await,
        }
    }
}

impl Showable for TextMessage {
    async fn show(&self) {
        let raw = self.text.to_string();
        let fixed_input = raw.replace("\\n", "\n").replace("\\", "");
        print_text(&fixed_input);
    }
}

impl Showable for TitleMessage {
    async fn show(&self) {
        let raw = self.title.to_string();
        let fixed_input = raw.replace("\\n", "\n").replace("\\", "");
        print!("title: ");
        print_text(&fixed_input);
    }
}

impl Showable for ImgMessage {
    async fn show(&self) {
        println!("running image interface");
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

impl Showable for Connect4 {
    async fn show(&self) {
        let name = &self.name;
        println!("{name}");
        for r in 0..6 {
            for c in 0..6 {
                let chip = self.grid.get(r).unwrap().row.get(c);
                match chip {
                    Some(c) => match c {
                        Chip::Red => print!("R"),
                        Chip::Yellow => print!("Y"),
                    },
                    None => print!("_"),
                }
                print!(" ");
            }
            println!();
        }
    }
}

#[derive(serde::Deserialize)]
struct Img {
    url: String,
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
