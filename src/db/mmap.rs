use std::collections::HashMap;

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::sync::RwLock;

use super::api::Engine;

struct BalanceAccount {
    uid: i64,
    balance: i64,
}

pub struct MMap {
    uid_map: DashMap<i64, BalanceAccount>,
}

impl MMap {
    pub fn new() -> Self {
        MMap {
            uid_map: DashMap::new(),
        }
    }
}

impl Engine for MMap {
    // add_money will add balance to uid account
    // if account do not exist, then just add a new one
    fn add_money(&self, uid: i64, amount: i64) {
        if let Some(mut account) = self.uid_map.get_mut(&uid) {
            account.balance += amount;
        } else {
            let account = BalanceAccount {
                uid,
                balance: amount,
            };
            self.uid_map.insert(uid, account);
        }
    }

    fn get_balance(&self, uid: i64) -> Result<i64> {
        let account = self
            .uid_map
            .get(&uid)
            .ok_or(anyhow!("can not find the account"))?;
        Ok(account.balance)
    }

    // transfer will transfer amount from 'from' account to 'to' account
    fn transfer(&self, from: i64, to: i64, amount: i64) -> Result<()> {
        let mut from_account = self
            .uid_map
            .get_mut(&from)
            .ok_or(anyhow!("can not find the account"))?;
        if from_account.balance < amount {
            return Err(anyhow!("insufficient balance"));
        }
        from_account.balance -= amount;

        let mut to_account = self
            .uid_map
            .get_mut(&to)
            .ok_or(anyhow!("can not find the account"))?;
        to_account.balance += amount;

        Ok(())
    }
}
