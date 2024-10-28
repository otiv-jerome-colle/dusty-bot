use slack_morphism::prelude::*;
use hyper::client::HttpConnector;
use hyper::Client;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use serde::{Deserialize, Serialize};
use tokio;

const LOCATION_FILE: &str = "location.json";

// Define a struct for storing Dusty's location
#[derive(Serialize, Deserialize, Default)]
struct Location {
    location: String,
}

// Helper functions to get and set the location in a JSON file
fn get_location() -> String {
    let mut file = File::open(LOCATION_FILE).unwrap_or_else(|_| File::create(LOCATION_FILE).unwrap());
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();

    let location: Location = serde_json::from_str(&data).unwrap_or_default();
    location.location
}

fn set_location(new_location: &str) {
    let location = Location {
        location: new_location.to_string(),
    };
    let data = serde_json::to_string(&location).unwrap();
    let mut file = OpenOptions::new().write(true).truncate(true).open(LOCATION_FILE).unwrap();
    file.write_all(data.as_bytes()).unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize HTTP connector with hyper
    let hyper_connector = HttpConnector::new();
    let hyper_client = Client::builder().build(hyper_connector);
    let client = SlackClient::new(hyper_client);

    // Setup OAuth access
    let slack_token_value = SlackApiToken::new("your-slack-bot-token".into());
    let session = SlackClientSession::new(client.clone(), slack_token_value);

    // Listen to incoming messages
    let listener = SlackEventListener::new().message_event(|event| async move {
        if let Some(text) = &event.text {
            // Handle "Where is Dusty?" query
            if text.trim() == "Where is Dusty?" {
                let location = get_location();
                let response_text = format!("Dusty is at {}", location);
                session.chat_post_message(event.channel.clone(), response_text).await.ok();
            }
            // Handle location update in "Dusty is at PX.ABC" format
            else if let Some(captures) = text.trim().strip_prefix("Dusty is at ") {
                set_location(captures);
                let response_text = format!("Got it! Dusty is now at {}", captures);
                session.chat_post_message(event.channel.clone(), response_text).await.ok();
            }
        }
        Ok(())
    });

    // Run the bot
    client.listen_with(listener).await?;
    Ok(())
}