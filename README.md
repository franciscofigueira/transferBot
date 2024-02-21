# transferBot
Telegram bot that sends user notifications about cryptocurrency transfers

## Quickstart

1. Setup your bot with [@botfather](https://t.me/botfather)

2. Create a `.env` file and insert the key obtained in the following format:
```
TELOXIDE_TOKEN="
``` 

3. Add desired chains to [state::CHAINS_INFO](./src/state.rs) and [state::AVAIALBLE_CHAINS](./src/state.rs).

4. Install [Rust](https://www.rust-lang.org/learn/get-started), and run:
```
cargo run
``` 

## Using Bot
To subscribe to token notifications, send `/subscribe`, and follow the proposed steps. If the proccess is sucesfull the bot will reply with `Everything is set.`

<img src="https://github.com/franciscofigueira/transferBot/extra/example_subscription.png" alt="drawing" width="400"/>

When a subscribed transfer is detected the bot will notify the user of the transaction details.

<img src="https://github.com/franciscofigueira/transferBot/extra/example_notification.png" alt="drawing" width="400"/>