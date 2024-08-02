use std::sync::{Arc, LazyLock};

use anyhow::Result;

use super::mmap::MMap;

pub trait Engine: Send + Sync {
    fn add_money(&self, uid: i64, amount: i64);
    fn get_balance(&self, uid: i64) -> Result<i64>;
    fn transfer(&self, from: i64, to: i64, amount: i64) -> Result<()>;
}

pub static MY_ENGINE: LazyLock<Arc<dyn Engine>> =
    LazyLock::new(|| Arc::new(MMap::new()));

pub fn add_money(uid: i64, amount: i64) {
    MY_ENGINE.add_money(uid, amount)
}

pub fn get_balance(uid: i64) -> Result<i64> {
    MY_ENGINE.get_balance(uid)
}

pub fn transfer(from: i64, to: i64, amount: i64) -> Result<()> {
    MY_ENGINE.transfer(from, to, amount)
}
