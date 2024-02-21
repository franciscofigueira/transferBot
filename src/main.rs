use eyre::Result;
use std::sync::Arc;
use teloxide::prelude::Bot;
use tokio::sync::RwLock;

mod bot;
mod chain_listener;
mod state;

use state::{State, CHAINS_INFO};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let api_key = dotenvy::var("TELOXIDE_TOKEN").unwrap();
    let bot = Bot::new(api_key);
    let state = Arc::new(RwLock::new(State::new()));

    for chain in CHAINS_INFO.values() {
        let clone_bot = bot.clone();
        let state_clone = state.clone();
        tokio::spawn(async move {
            chain_listener::listener(chain, state_clone, clone_bot).await;
        });
    }

    bot::run(bot, state).await;

    Ok(())
}
