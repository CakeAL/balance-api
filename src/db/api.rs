use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, RwLock},
};

use anyhow::Result;

use super::mmap::MMap;

pub trait Engine: Send + Sync {
    fn add_money(&mut self, uid: i64, amount: i64);
    fn get_balance(&self, uid: i64) -> Result<i64>;
    fn transfer(&mut self, from: i64, to: i64, amount: i64) -> Result<()>;
    fn get_all_balance(&self) -> HashMap<i64, i64>;
}

pub static MY_ENGINE: LazyLock<Arc<RwLock<dyn Engine>>> =
    LazyLock::new(|| Arc::new(RwLock::new(MMap::new())));

pub fn add_money(uid: i64, amount: i64) {
    MY_ENGINE.write().unwrap().add_money(uid, amount)
}

pub fn get_balance(uid: i64) -> Result<i64> {
    MY_ENGINE.write().unwrap().get_balance(uid)
}

pub fn transfer(from: i64, to: i64, amount: i64) -> Result<()> {
    MY_ENGINE.write().unwrap().transfer(from, to, amount)
}

pub fn get_all_balance() -> HashMap<i64, i64> {
    MY_ENGINE.read().unwrap().get_all_balance()
}