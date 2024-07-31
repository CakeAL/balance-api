use std::collections::HashMap;

use anyhow::{anyhow, Result};
use std::sync::RwLock;

use super::api::Engine;

struct BalanceAccount {
    uid: i64,
    balance: RwLock<i64>,
}

pub struct MMap {
    uid_map: HashMap<i64, BalanceAccount>,
}

impl MMap {
    pub fn new() -> Self {
        MMap {
            uid_map: HashMap::new(),
        }
    }
}

impl Engine for MMap {
    // add_money will add balance to uid account
    // if account do not exist, then just add a new one
    fn add_money(&mut self, uid: i64, amount: i64) {
        if let Some(account) = self.uid_map.get_mut(&uid) {
            *account.balance.write().unwrap() += amount;
        } else {
            let account = BalanceAccount {
                uid,
                balance: RwLock::new(amount),
            };
            self.uid_map.insert(uid, account);
        }
    }

    fn get_balance(&self, uid: i64) -> Result<i64> {
        let account = self
            .uid_map
            .get(&uid)
            .ok_or(anyhow!("can not find the account"))?;
        Ok(*account.balance.read().unwrap())
    }

    // transfer will transfer amount from 'from' account to 'to' account
    fn transfer(&mut self, from: i64, to: i64, amount: i64) -> Result<()> {
        let from_account = self
            .uid_map
            .get_mut(&from)
            .ok_or(anyhow!("can not find the account"))?;
        if *from_account.balance.read().unwrap() < amount {
            return Err(anyhow!("insufficient balance"));
        }
        *from_account.balance.write().unwrap() -= amount;

        let to_account = self
            .uid_map
            .get_mut(&to)
            .ok_or(anyhow!("can not find the account"))?;
        *to_account.balance.write().unwrap() += amount;

        Ok(())
    }

    fn get_all_balance(&self) -> HashMap<i64, i64> {
        self.uid_map
            .iter()
            .map(|(uid, account)| (*uid, *account.balance.read().unwrap()))
            .collect()
    }
}
