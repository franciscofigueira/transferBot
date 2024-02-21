use ethers::types::Address;
use eyre::{eyre, Ok, Result};
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use teloxide::types::ChatId;

lazy_static! {
    pub static ref CHAINS_INFO: HashMap<u32, ChainInfo> = {
        let mut m = HashMap::new();
        // m.insert(
        //     0,
        //     ChainInfo {
        //         id: 0,
        //         name: "Local",
        //         scanner_url: "https://sepolia.etherscan.io/",
        //         ws: "ws://localhost:8545",
        //     },
        // );

        m.insert(
            11155111,
            ChainInfo {
                id: 11155111,
                name: "ETH Sepolia",
                scanner_url: "https://sepolia.etherscan.io/",
                ws: "wss://sepolia.gateway.tenderly.co",
            },
        );
        m
    };
    pub static ref AVAILABLE_CHAINS: HashMap<&'static str, u32> = {
        let mut m = HashMap::new();
        // m.insert("Local", 0);
        m.insert("ETH Sepolia", 11155111);
        m
    };
}

pub struct ChainInfo {
    pub id: u32,
    pub name: &'static str,
    pub scanner_url: &'static str,
    pub ws: &'static str,
}

#[derive(Debug)]
pub struct Subscription {
    chain_id: u32,
    token_address: Address,
    token_sender_receiver: Address,
}

#[derive(Debug)]
pub struct State {
    //chain Id -> token address -> user address -> subscribed users
    pub subs: HashMap<u32, HashMap<Address, HashMap<Address, HashSet<ChatId>>>>,
    pub user_subs: HashMap<ChatId, Vec<Subscription>>,
    //chain Id -> token Address -> (name, symbol,decimals)
    pub cached_token_metadata: HashMap<u32, HashMap<Address, (String, String, u8)>>,
}

impl State {
    pub fn new() -> Self {
        let mut subs: HashMap<u32, HashMap<Address, HashMap<Address, HashSet<ChatId>>>> =
            HashMap::new();
        let mut cached_token_metadata: HashMap<u32, HashMap<Address, (String, String, u8)>> =
            HashMap::new();
        for id in CHAINS_INFO.keys() {
            subs.insert(*id, HashMap::new());
            cached_token_metadata.insert(*id, HashMap::new());
        }

        Self {
            subs,
            user_subs: HashMap::new(),
            cached_token_metadata,
        }
    }

    pub fn get_token_metadata(
        &self,
        chain_id: &u32,
        token_address: &Address,
    ) -> Option<&(String, String, u8)> {
        self.cached_token_metadata
            .get(chain_id)
            .expect("chain will exist")
            .get(token_address)
    }

    pub fn insert_token_metadata(
        &mut self,
        chain_id: &u32,
        token_address: Address,
        token_name: String,
        token_symbol: String,
        decimals: u8,
    ) {
        self.cached_token_metadata
            .get_mut(chain_id)
            .expect("chain will exist")
            .insert(token_address, (token_name, token_symbol, decimals));
    }

    pub fn get_user_subscriptions_formated(&self, user: &ChatId) -> Option<String> {
        if let Some(subs) = self.user_subs.get(user) {
            if subs.len() != 0 {
                return Some(format!(
                    "{:?}",
                    subs.iter()
                        .enumerate()
                        .collect::<Vec<(usize, &Subscription)>>()
                ));
            }
        }
        None
    }

    pub fn remove_sub(&mut self, user: &ChatId, index: usize) -> Result<Subscription> {
        if let Some(user_subs) = self.user_subs.get_mut(user) {
            if index < user_subs.len() {
                let subscription = user_subs.remove(index);
                let subscribed_users = self
                    .subs
                    .get_mut(&subscription.chain_id)
                    .expect("chain will exist")
                    .get_mut(&subscription.token_address)
                    .expect("token will exist")
                    .get_mut(&subscription.token_sender_receiver)
                    .expect("user will exist");
                subscribed_users.remove(user);
                Ok(subscription)
            } else {
                Err(eyre!("index out of bounds"))
            }
        } else {
            Err(eyre!("No subs"))
        }
    }

    pub fn get_sub_users(
        &self,
        chain_id: &u32,
        token_address: &Address,
        token_sender_receiver: &Address,
    ) -> Option<&HashSet<ChatId>> {
        if let Some(tokens) = self.subs.get(chain_id) {
            if let Some(addresses) = tokens.get(token_address) {
                return addresses.get(token_sender_receiver);
            }
        }
        None
    }

    pub fn insert_sub(
        &mut self,
        chain_id: u32,
        token_address: Address,
        token_sender_receiver: Address,
        user_id: ChatId,
    ) {
        let tokens = self.subs.get_mut(&chain_id).expect("chain will exist");

        match tokens.get_mut(&token_address) {
            Some(addresses) => match addresses.get_mut(&token_sender_receiver) {
                Some(s) => {
                    if !s.insert(user_id) {
                        return;
                    }
                }
                None => {
                    let mut v = HashSet::new();
                    v.insert(user_id);
                    addresses.insert(token_sender_receiver, v);
                }
            },
            None => {
                tokens.insert(
                    token_address,
                    Self::init_user(token_sender_receiver, user_id),
                );
            }
        }
        self.add_user_sub(chain_id, token_address, token_sender_receiver, user_id);
    }

    fn init_user(
        token_sender_receiver: Address,
        user_id: ChatId,
    ) -> HashMap<Address, HashSet<ChatId>> {
        let mut m = HashMap::new();
        let mut v = HashSet::new();
        v.insert(user_id);
        m.insert(token_sender_receiver, v);
        m
    }

    fn add_user_sub(
        &mut self,
        chain_id: u32,
        token_address: Address,
        token_sender_receiver: Address,
        user_id: ChatId,
    ) {
        if let Some(subs) = self.user_subs.get_mut(&user_id) {
            subs.push(Subscription {
                chain_id,
                token_address,
                token_sender_receiver,
            });
        } else {
            self.user_subs.insert(
                user_id,
                vec![Subscription {
                    chain_id,
                    token_address,
                    token_sender_receiver,
                }],
            );
        }
    }
}
