# transferBot
Telegram bot that sends user notifications about cryptocurrency transfers

## Quickstart

1. Setup your bot with [@botfather](https://t.me/botfather)

2. Create a `.env` file and insert the token obtained with [@botfather](https://t.me/botfather), in the following format:
```
TELOXIDE_TOKEN=<Your token here>
``` 

3. Add desired chains to [state::CHAINS_INFO](./src/state.rs) and [state::AVAILABLE_CHAINS](./src/state.rs).

4. Install [Rust](https://www.rust-lang.org/learn/get-started), and run:
```
cargo run
``` 

## Using the Bot
### Commands
#### `/subscribe`
Iniates the subscription proccess to receive notification upon target token transfer, from/to specific user.
#### `/cancel`
Cancel subscription proccess.

#### `/subs` 
List all current subscriptions and their id.

#### `/unsubscribe <sub_id>`
Unsubscribe notifications of subscription.

#### `/help`
List all available commands.


### Example
To subscribe to token notifications, send `/subscribe`, and follow the proposed steps. If the proccess is sucesfull the bot will reply with `Everything is set.`

<img src="https://github.com/franciscofigueira/transferBot/blob/main/extra/example_subscription.png?raw=true" alt="drawing" width="400"/>

When a subscribed transfer is detected the bot will notify the user of the transaction details.

<img src="https://github.com/franciscofigueira/transferBot/blob/main/extra/example_notification.png?raw=true" alt="drawing" width="400"/>


