use std::sync::LazyLock;

use dashmap::DashSet;

pub struct UuidCache {
    batch_pay: DashSet<String>,
    trade: DashSet<String>,
}

pub static UUID_CACHE_INSTANCE: LazyLock<UuidCache> = LazyLock::new(|| UuidCache {
    batch_pay: DashSet::new(),
    trade: DashSet::new(),
});

pub fn check_and_add_batch_pay(uuid: String) -> bool {
    match UUID_CACHE_INSTANCE.batch_pay.contains(&uuid) {
        true => false,
        false => {
            UUID_CACHE_INSTANCE.batch_pay.insert(uuid);
            true
        }
    }
}

pub fn check_and_add_trade(uuid: String) -> bool {
    match UUID_CACHE_INSTANCE.trade.contains(&uuid) {
        true => false,
        false => {
            UUID_CACHE_INSTANCE.trade.insert(uuid);
            true
        }
    }
}
