use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::GLOBAL_CONFIG;

#[derive(Serialize)]
struct GetFundJson {
    #[serde(rename = "transactionId")]
    transaction_id: String,
    uid: i64,
    amount: f64,
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

#[derive(Debug, Deserialize, Serialize)]
pub struct Fund {
    pub uid: i64,
    pub amount: f64,
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
    println!("Response status code: {}", status);
    println!("Response body: {}", body);
    Ok(())
}

pub async fn get_all_fund(uid: i64) -> Result<i64> {
    let config = &*GLOBAL_CONFIG;
    let timeout = Duration::from_millis(config.server.request_timeout as u64);

    let (mut pre, mut ans) = (500000i64, 0i64);
    let mut unique_id = Uuid::new_v4().to_string();
    while pre >= 1 {
        match tokio::time::timeout(timeout, get_pay(uid, pre, unique_id.clone())).await {
            Ok(res) => match res {
                Err(e) => {
                    if e.to_string().contains("Request failed with status code:") {
                        continue;
                    } else {
                        return Err(e);
                    }
                }
                Ok(code) => match code {
                    200 => {
                        ans += pre;
                        unique_id = Uuid::new_v4().to_string();
                        continue;
                    }
                    501 => {
                        pre /= 2;
                        unique_id = Uuid::new_v4().to_string();
                        continue;
                    }
                    404 => {
                        return Err(anyhow!("not found account by uid: {}", uid));
                    }
                    _ => continue,
                },
            },
            Err(_) => continue, // timeout
        }
    }

    Ok(ans)
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
