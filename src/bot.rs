use ethers::{
    prelude::abigen,
    providers::{Provider, Ws},
    types::Address,
};
use eyre::{eyre, Result};
use std::{str::FromStr, sync::Arc};
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler,
    },
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    utils::command::BotCommands,
};
use tokio::sync::RwLock;

use crate::state::{State, AVAILABLE_CHAINS, CHAINS_INFO};

type MyDialogue = Dialogue<ChatState, InMemStorage<ChatState>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
enum ChatState {
    #[default]
    Start,
    ReceiveChainId,
    ReceiveTokenAddress {
        chain_id: u32,
    },
    ReceiveUser {
        chain_id: u32,
        token_address: Address,
    },
}

#[derive(BotCommands, Clone, Debug)]
#[command(description = "Commands:", rename_rule = "lowercase")]
enum Command {
    #[command(description = "Display all commands")]
    Help,
    #[command(description = "Subscribe to receive notifications of token transfers")]
    Subscribe,
    #[command(
        description = "Unsubscribe of token transfer, by passing in the id. Ids can be obtained in the /subs command"
    )]
    Unsubscribe(u32),
    #[command(description = "Display all current token subscriptions")]
    Subs,
    #[command(description = "Cancel susbscription process")]
    Cancel,
}

pub async fn run(bot: Bot, state: Arc<RwLock<State>>) {
    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![state, InMemStorage::<ChatState>::new()])
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![ChatState::Start]
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Subscribe].endpoint(subscribe))
                .branch(case![Command::Unsubscribe(id)].endpoint(unsubscribe))
                .branch(case![Command::Subs].endpoint(subs)),
        )
        .branch(case![Command::Cancel].endpoint(cancel));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![ChatState::ReceiveTokenAddress { chain_id }].endpoint(receive_token_address))
        .branch(
            case![ChatState::ReceiveUser {
                chain_id,
                token_address
            }]
            .endpoint(receive_user),
        )
        .branch(dptree::endpoint(invalid_state));

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![ChatState::ReceiveChainId].endpoint(receive_chain_id));

    dialogue::enter::<Update, InMemStorage<ChatState>, ChatState, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Unable to handle the message. Type /help to see the usage.",
    )
    .await?;
    Ok(())
}

async fn cancel(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Cancelling the subscription process.")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn subscribe(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Let's start! Select desired chain.")
        .await?;
    let chains = AVAILABLE_CHAINS
        .keys()
        .map(|chain| InlineKeyboardButton::callback(chain.to_string(), chain.to_string()));
    bot.send_message(msg.chat.id, "Select a chain:")
        .reply_markup(InlineKeyboardMarkup::new([chains]))
        .await?;
    dialogue.update(ChatState::ReceiveChainId).await?;
    Ok(())
}

async fn unsubscribe(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<RwLock<State>>,
) -> HandlerResult {
    match cmd {
        Command::Unsubscribe(index) => {
            let mut state = state.write().await;
            if let Ok(subscription) = state.remove_sub(&msg.chat.id, index as usize) {
                bot.send_message(
                    msg.chat.id,
                    format!("Succesfully unsubscribed from {:?}", subscription),
                )
                .await?;
            } else {
                bot.send_message(msg.chat.id, format!("Error invalid index."))
                    .await?;
            }
        }
        _ => {
            panic!("unexpected state");
        }
    }

    Ok(())
}

async fn subs(bot: Bot, msg: Message, state: Arc<RwLock<State>>) -> HandlerResult {
    let state = state.read().await;
    let subs = state.get_user_subscriptions_formated(&msg.chat.id);
    if let Some(subs) = subs {
        bot.send_message(msg.chat.id, format!("Your subs {:?}", subs))
            .await?;
    } else {
        bot.send_message(msg.chat.id, format!("You currently have no subs"))
            .await?;
    }
    Ok(())
}

async fn receive_chain_id(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    if let Some(chain_name) = q.data {
        let chain_id = AVAILABLE_CHAINS.get(chain_name.as_str()).unwrap();
        bot.send_message(
            dialogue.chat_id(),
            format!(
                "You've selected {} chain.\n Please insert the token address.",
                chain_name
            ),
        )
        .await?;
        dialogue
            .update(ChatState::ReceiveTokenAddress {
                chain_id: chain_id.to_owned(),
            })
            .await?;
    }
    Ok(())
}

async fn receive_token_address(
    bot: Bot,
    dialogue: MyDialogue,
    chain_id: u32, // Available from `ChatState::ReceiveChainID`.
    state: Arc<RwLock<State>>,
    msg: Message,
) -> HandlerResult {
    match msg.text().map(ToOwned::to_owned) {
        Some(token_address) => {
            if let Ok(token_address) = Address::from_str(&token_address) {
                let state_read = state.read().await;
                if let Some((name, symbol, _)) =
                    state_read.get_token_metadata(&chain_id, &token_address)
                {
                    let response = format!(
                        "Target token has name: {}, and symbol: {} .\n Please insert the user address.",
                        name, symbol
                    );
                    bot.send_message(msg.chat.id, response).await?;
                    dialogue
                        .update(ChatState::ReceiveUser {
                            chain_id,
                            token_address,
                        })
                        .await?;
                } else if let Ok((name, symbol, decimals)) =
                    fetch_token_metadata(CHAINS_INFO.get(&chain_id).unwrap().ws, token_address)
                        .await
                {
                    drop(state_read);

                    let mut state = state.write().await;
                    state.insert_token_metadata(
                        &chain_id,
                        token_address,
                        name.clone(),
                        symbol.clone(),
                        decimals,
                    );
                    let response = format!(
                        "Target token has name: {}, and symbol: {} .\n Please insert the user address.",
                        name, symbol
                    );
                    bot.send_message(msg.chat.id, response).await?;
                    dialogue
                        .update(ChatState::ReceiveUser {
                            chain_id,
                            token_address,
                        })
                        .await?;
                } else {
                    bot.send_message(msg.chat.id, "Address given does not correspond to a token, please insert an ERC20 token address.")
                        .await?;
                }
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Invalid address. Please insert a valid address.",
                )
                .await?;
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Please, send the token address.")
                .await?;
        }
    }
    Ok(())
}

async fn receive_user(
    bot: Bot,
    dialogue: MyDialogue,
    state: Arc<RwLock<State>>,
    (chain_id, token_address): (u32, Address), // Available from `ChatState::ReceiveTokenAddress`.
    msg: Message,
) -> HandlerResult {
    match msg.text().map(ToOwned::to_owned) {
        Some(user_address) => {
            let mut state = state.write().await;

            if let Ok(user_address) = Address::from_str(&user_address) {
                state.insert_sub(chain_id, token_address, user_address, msg.chat.id);

                bot.send_message(msg.chat.id, "Everything is set.").await?;
                dialogue.exit().await?
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Invalid address. Please insert a valid address.",
                )
                .await?;
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Please send an address")
                .await?;
        }
    }

    Ok(())
}

async fn fetch_token_metadata(rpc: &str, token_address: Address) -> Result<(String, String, u8)> {
    abigen!(
        IERC20,
        r#"[
            function symbol() public view returns (string memory)
            function name() public view returns (string memory)
            function decimals() public view virtual returns (uint8)
        ]"#,
    );
    let provider = Provider::<Ws>::connect(rpc).await?;
    let client = Arc::new(provider);
    let contract = IERC20::new(token_address, client);
    if let Ok(name) = contract.name().call().await {
        if let Ok(symbol) = contract.symbol().call().await {
            if let Ok(decimals) = contract.decimals().call().await {
                return Ok((name, symbol, decimals));
            }
        }
    }
    return Err(eyre!("Contract Call failed"));
}
