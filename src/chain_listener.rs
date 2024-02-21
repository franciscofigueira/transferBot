use ethers::{
    abi::AbiDecode,
    providers::{Middleware, Provider, StreamExt, Ws},
    types::{Address, Filter, U256},
};
use std::sync::Arc;
use teloxide::prelude::*;
use tokio::sync::RwLock;

use crate::state::{ChainInfo, State};

#[derive(Debug)]
struct TransferEvent {
    chain_name: String,
    tx_on_scanner: String,
    token_on_scanner: String,
    name: String,
    from: Address,
    sender_on_scanner: String,
    to: Address,
    receiver_on_scanner: String,
    amount: String,
}

impl TransferEvent {
    fn format(&self) -> String {
        format!(
            "
        Tokens transfered on {}
        Token: [{}]({})
        From: [{:#x}]({})
        To: [{:#x}]({})
        Amount: {}
        View tx on [explorer]({})
        ",
            self.chain_name,
            self.name,
            self.token_on_scanner,
            self.from,
            self.sender_on_scanner,
            self.to,
            self.receiver_on_scanner,
            self.amount,
            self.tx_on_scanner
        )
    }
}

pub async fn listener(chain: &ChainInfo, state: Arc<RwLock<State>>, bot: Bot) {
    let client = Provider::<Ws>::connect(chain.ws).await.unwrap();

    let erc20_transfer_filter = Filter::new().event("Transfer(address,address,uint256)");

    let mut stream = client.subscribe_logs(&erc20_transfer_filter).await.unwrap();

    while let Some(log) = stream.next().await {
        let state = state.read().await;
        if let Some((name, _, decimals)) = state.get_token_metadata(&chain.id, &log.address) {
            let tx_on_scanner = format!(
                "{}tx/{:#x}",
                chain.scanner_url,
                log.transaction_hash.unwrap()
            );
            let sender_on_scanner = format!(
                "{}address/{:#x}",
                chain.scanner_url,
                Address::from(log.topics[1])
            );
            let receiver_on_scanner = format!(
                "{}address/{:#x}",
                chain.scanner_url,
                Address::from(log.topics[2])
            );
            let token_on_scanner = format!("{}address/{:#x}", chain.scanner_url, log.address);
            let amount_formated = format_amount(
                U256::decode(log.data).unwrap_or_else(|_| U256::from_big_endian(&[0])),
                *decimals,
            );
            let parsed_event = TransferEvent {
                chain_name: chain.name.to_owned(),
                tx_on_scanner,
                token_on_scanner,
                name: name.clone(),
                from: Address::from(log.topics[1]),
                sender_on_scanner,
                to: Address::from(log.topics[2]),
                receiver_on_scanner,
                amount: amount_formated,
            };
            let parsed_event = parsed_event.format();

            if let Some(users) =
                state.get_sub_users(&chain.id, &log.address, &Address::from(log.topics[2]))
            {
                for user in users {
                    let clone_bot = bot.clone();
                    let user = user.clone();
                    let message = parsed_event.clone();
                    tokio::spawn(async move {
                        clone_bot
                            .send_message(user, message)
                            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                            .await
                            .unwrap();
                    });
                }
            }
            if let Some(users) =
                state.get_sub_users(&chain.id, &log.address, &Address::from(log.topics[1]))
            {
                for user in users {
                    let clone_bot = bot.clone();
                    let user = user.clone();
                    let message = parsed_event.clone();
                    tokio::spawn(async move {
                        clone_bot
                            .send_message(user, message)
                            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                            .await
                            .unwrap();
                    });
                }
            }
        }
    }
}

fn format_amount(amount: U256, decimals: u8) -> String {
    let units = decimals as usize;
    let exp10 = U256::exp10(units);

    let integer = amount / exp10;
    let decimals = (amount % exp10).to_string();

    format!("{integer}\\.{decimals:0>units$}")
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_format_amount() {
        //131.55 ether
        let amount = U256::from_dec_str("131550000000000000000").unwrap();
        let formated = format_amount(amount, 18);
        assert_eq!(formated, "131\\.550000000000000000".to_string());

        //1.2
        let amount = U256::from_dec_str("1200000").unwrap();
        let formated = format_amount(amount, 6);
        assert_eq!(formated, "1\\.200000".to_string());
    }
}
