use bens_chat_shared::{Bot, Message};
fn main() {
    Bot::no_responce(|msg: Message| {
        println!("{}: {}", msg.sender_name, msg.content);
    });
}
