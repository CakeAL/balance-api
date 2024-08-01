use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::GLOBAL_CONFIG;

#[derive(Serialize)]
struct GetFundJson {
    #[serde(rename = "transactionId")]
    transaction_id: String,
    uid: i64,
    amount: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Fund {
    pub uid: i64,
    pub amount: f64,
}

#[derive(Deserialize)]
struct GetFundResponse {
    code: i32,
    #[serde(rename = "requestId")]
    request_id: String,
    msg: String,
    data: String,
}

// 不保证正确
pub async fn get_pay(uid: i64, amount: i64, unique_id: String) -> Result<i32> {
    tracing::info!("GETPAY uid: {uid}, amount: {amount}, uniqueID: {unique_id}");
    let data = GetFundJson {
        transaction_id: unique_id,
        uid,
        amount: amount as f64 / 100.0,
    };
    let uuid = Uuid::new_v4().to_string();
    let json_data = serde_json::json!(data);
    let response = Client::new()
        .post("http://example.com")
        .header("Content-Type", "application/json")
        .header("X-KSY-REQUEST-ID", &uuid)
        .header("X-KSY-KINGSTAR-ID", "20004")
        .body(json_data.to_string())
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;

    // 打印响应体
    // println!("Response status code: {}", status);
    // println!("Response body: {}", body);

    match status {
        StatusCode::OK => {}
        StatusCode::GATEWAY_TIMEOUT => {
            return Err(anyhow!("Request failed with status code: {}", status))
        }
        _ => return Err(anyhow!("Error getting fund: {}", status)),
    }

    let result: GetFundResponse = serde_json::from_str(&body)?;
    if result.request_id != uuid {
        return Err(anyhow!("Error getting fund: {} {}", status, body));
    }

    Ok(result.code)
}

pub async fn init_funds(list: Vec<Fund>) -> Result<()> {
    let json_data = json!(list);
    let response = Client::new()
        .post("http://example.com")
        .header("Content-Type", "application/json")
        .header("X-KSY-REQUEST-ID", "1")
        .header("X-KSY-KINGSTAR-ID", "20004")
        .body(json_data.to_string())
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    // 打印响应体
    tracing::info!("Response status code: {}", status);
    tracing::info!("Response body: {}", body);
    Ok(())
}

pub async fn get_all_fund(uid: i64) -> Result<i64> {
    let (mut pre, mut ans) = (500000i64, 0i64);
    println!("before get all one amount");
    ans += get_all_one_amount(uid, 1000000, 100).await;
    while pre >= 1 {
        ans += get_all_one_amount(uid, pre, 2).await;
        pre /= 2;
    }

    Ok(ans)
}

async fn get_all_one_amount(uid: i64, amount: i64, max_parallel: usize) -> i64 {
    let ans = Arc::new(RwLock::new(0i64));
    let mut handles = Vec::new();
    for _ in 0..max_parallel {
        let ans = ans.clone();
        let handler = tokio::spawn(async move {
            let signal_get = singal_get_pay(uid, amount).await;
            println!("signal_get: {signal_get}");
            *ans.write().await += signal_get;
        });
        handles.push(handler);
    }
    for handle in handles {
        handle.await.unwrap();
    }
    *ans.to_owned().read().await
}

async fn singal_get_pay(uid: i64, amount: i64) -> i64 {
    let mut ans = 0i64;
    let mut unique_id = Uuid::new_v4().to_string();
    let config = &*GLOBAL_CONFIG;
    let timeout = Duration::from_millis(config.server.request_timeout as u64);
    loop {
        match tokio::time::timeout(timeout, get_pay(uid, amount, unique_id.clone())).await {
            Ok(res) => match res {
                Err(_) => {}
                Ok(code) => match code {
                    200 => {
                        ans += amount;
                        unique_id = Uuid::new_v4().to_string();
                        continue;
                    }
                    501 => return ans,
                    404 => return 0,
                    _ => continue,
                },
            },
            Err(_) => {
                println!("timeout");
                continue;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_pay() {
        let uid = 600001i64;
        let amount = 1i64;
        let res = get_pay(uid, amount, "aaaaa".into()).await;
        dbg!(res.unwrap());
    }

    #[tokio::test]
    async fn test_get_all_fund() {
        let funds = vec![
            Fund {
                uid: 600001,
                amount: 88.91,
            },
            Fund {
                uid: 600002,
                amount: 10000.93,
            },
        ];
        let res = init_funds(funds).await;
        dbg!(res.unwrap());
        let res = get_all_fund(600002).await;
        dbg!(res.unwrap());
    }

    #[tokio::test]
    async fn test_init_fund() {
        let funds = vec![
            Fund {
                uid: 600001,
                amount: 88.91,
            },
            Fund {
                uid: 600002,
                amount: 10000.93,
            },
        ];
        let res = init_funds(funds).await;
        dbg!(res.unwrap());
    }
}
