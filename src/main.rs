mod agent;

use agent::KenAgent;
use axum::http::response;
use rig::completion::Chat;

#[tokio::main]
async fn main() {

    let agent = KenAgent::default();
    let message = "How are you?";
    match agent.chat(message, Vec::new()).await {
        Ok(response) => {
            println!("{}", response);
        },
        Err(err) => {
            eprintln!("Error: {}", err);
        }

    };
}
