use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use awaitgroup::WaitGroup;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    sync::mpsc::{self, Sender},
    task, time,
};
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
pub async fn get_pay(uid: i64, amount: i64, unique_id: String, tx: Sender<i32>) {
    println!("GETPAY uid: {uid}, amount: {amount}, uniqueID: {unique_id}");
    let data = GetFundJson {
        transaction_id: unique_id,
        uid,
        amount: amount as f64 / 100.0,
    };
    let uuid = Uuid::new_v4().to_string();
    let json_data = serde_json::json!(data);
    let response = Client::new()
        .post(&*GLOBAL_CONFIG.urls.get_pay)
        .header("Content-Type", "application/json")
        .header("X-KSY-REQUEST-ID", &uuid)
        .header("X-KSY-KINGSTAR-ID", "20004")
        .body(json_data.to_string())
        .send()
        .await;

    if let Err(e) = response {
        tracing::error!("{}", e.to_string());
        let _ = tx.send(0).await;
        return;
    }
    let response = response.unwrap();
    let status = response.status();
    let body = response.text().await;
    if let Err(e) = body {
        tracing::error!("{}", e.to_string());
        let _ = tx.send(0).await;
        return;
    }

    // 打印响应体
    // println!("Response status code: {}", status);
    // println!("Response body: {}", body);

    match status {
        StatusCode::OK => {}
        // StatusCode::GATEWAY_TIMEOUT => {
        //     // return Err(anyhow!("Request failed with status code: {}", status))
        //     tracing::error!("Request failed with status code: {}", status);
        //     return;
        // }
        _ => {
            // return Err(anyhow!("Error getting fund: {}", status))
            let _ = tx.send(1).await;
            return;
        }
    }

    let result = serde_json::from_str::<GetFundResponse>(&body.unwrap());
    if let Err(e) = result {
        tracing::error!("{}", e.to_string());
        let _ = tx.send(0).await;
        return;
    }
    let result = result.unwrap();
    if result.request_id != uuid {
        // return Err(anyhow!("Error getting fund: {} {}", status, body));
        let _ = tx.send(1).await;
        return;
    }

    let _ = tx.send(result.code).await;
}

pub async fn init_funds(list: Vec<Fund>) -> Result<()> {
    let json_data = json!(list);
    let response = Client::new()
        .post(&*GLOBAL_CONFIG.urls.init_funds)
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
    let (mut pre, mut ans) = (500000i64, 0i64);
    println!("before get all one amount");
    ans += get_all_one_amount(uid, 1000000, 30).await;
    while pre >= 1 {
        ans += get_all_one_amount(uid, pre, 2).await;
        pre /= 2;
    }

    Ok(ans)
}

async fn get_all_one_amount(uid: i64, amount: i64, max_parallel: usize) -> i64 {
    let ans = Arc::new(Mutex::new(0));
    let (tx_done, mut rx_done) = mpsc::channel(500);
    let mut wg = WaitGroup::new();
    for i in 1..=max_parallel {
        if let Ok(_) = rx_done.try_recv() {
            break;
        }
        if max_parallel > 2 {
            if i < 30 && i != 1 {
                time::sleep(Duration::from_millis(10)).await;
            }
        }
        let ans = ans.clone();
        let tx_done = tx_done.clone();
        let worker = wg.worker();
        task::spawn(async move {
            *ans.lock().unwrap() += singal_get_pay(uid, amount).await;
            tx_done.send(true).await.unwrap();
            worker.done();
        });
    }
    wg.wait().await;
    *ans.to_owned().lock().unwrap()
}

async fn singal_get_pay(uid: i64, amount: i64) -> i64 {
    let config = &*GLOBAL_CONFIG;
    let timeout = Duration::from_millis(config.server.request_timeout as u64);
    let mut ans = 0i64;
    let mut unique_id = Uuid::new_v4().to_string();

    loop {
        // 超时/或其他原因重试
        let (tx, mut rx) = mpsc::channel(100);
        let tx_clone = tx.clone();
        let unique_id_1 = unique_id.clone();
        task::spawn(get_pay(uid, amount, unique_id_1, tx_clone));

        tokio::select! {
            Some(code) = rx.recv() => {
                match code {
                    200 => {
                        ans += amount;
                        unique_id = Uuid::new_v4().to_string();
                        continue;
                    }
                    501 => return ans,
                    404 => return 0,
                    _ => continue,
                }
            },
            _ = time::sleep(timeout) => {
                // println!("Timeout!");
                if let Some(_) = rx.recv().await {}
                continue;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[tokio::test]
    // async fn test_get_pay() {
    //     let uid = 600001i64;
    //     let amount = 1i64;
    //     let res = get_pay(uid, amount, "aaaaa".into()).await;
    //     dbg!(res.unwrap());
    // }

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
        let res = get_all_fund(600004).await;
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
