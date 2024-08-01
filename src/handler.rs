use std::time::Duration;

use axum::{
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{task, time::Instant};
use uuid::Uuid;

use crate::{
    db,
    fund::{get_all_fund, Fund},
    uuid_cache,
};

#[derive(Deserialize, Serialize)]
pub struct BatchPayJson {
    #[serde(rename = "batchPayId")]
    batch_pay_id: String,
    #[serde(rename = "uids")]
    uids: Vec<i64>,
}

#[derive(Serialize)]
struct QueryUserAmountDataResponse {
    code: i32,
    msg: String,
    #[serde(rename = "requestId")]
    request_id: String,
    data: Vec<Fund>,
}

#[derive(Serialize)]
struct FinishJson {
    #[serde(rename = "batchPayId")]
    batch_pay_id: String,
}

#[derive(Serialize, Deserialize)]
struct UserTradeJson {
    #[serde(rename = "sourceUid")]
    source_uid: i64,
    #[serde(rename = "targetUid")]
    target_uid: i64,
    amount: f64,
}

pub async fn batch_pay(header: HeaderMap, Json(body): Json<BatchPayJson>) -> impl IntoResponse {
    let batch_pay_id = body.batch_pay_id.to_owned();
    if !uuid_cache::check_and_add_batch_pay(batch_pay_id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "batchPayId already exist"})),
        )
            .into_response();
    }

    // 开一个异步任务
    task::spawn(do_batch_pay(body));
    let request_id = match header.get("X-KSY-REQUEST-ID") {
        Some(value) => value.to_str().unwrap().to_string(),
        None => "".to_string(),
    };

    (
        StatusCode::OK,
        Json(json!({"msg": "ok", "code": 200, "requestId": request_id})),
    )
        .into_response()
}

pub async fn user_trade(header: HeaderMap, body_raw: String) -> impl IntoResponse {
    // TODO: make sure each request_id will only do once
    let request_id = match header.get("X-KSY-REQUEST-ID") {
        Some(value) => value.to_str().unwrap().to_string(),
        None => "".to_string(),
    };
    if !uuid_cache::check_and_add_trade(request_id.clone()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "requestId already exist"})),
        )
            .into_response();
    }
    let body: UserTradeJson = match serde_json::from_str(&body_raw) {
        Ok(body) => body,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid JSON"
                })),
            )
                .into_response();
        }
    };

    if let Err(err) = db::api::transfer(
        body.source_uid,
        body.target_uid,
        (body.amount * 100.0).round() as i64,
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": err.to_string()})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({"msg": "ok", "code": 200, "requestId": request_id})),
    )
        .into_response()
}

pub async fn query_user_amount(header: HeaderMap, Json(body): Json<Vec<i64>>) -> impl IntoResponse {
    let mut data = vec![];
    for uid in body {
        let amount = db::api::get_balance(uid).unwrap_or(0);
        data.push(Fund {
            uid,
            amount: amount as f64 / 100.0,
        });
    }
    let request_id = match header.get("X-KSY-REQUEST-ID") {
        Some(value) => value.to_str().unwrap().to_string(),
        None => "".to_string(),
    };
    (
        StatusCode::OK,
        Json(json!(QueryUserAmountDataResponse {
            code: 200,
            msg: "ok".to_string(),
            request_id,
            data,
        })),
    )
}

pub async fn batch_pay_finish(req_uuid: String, request_id: String) -> i32 {
    let data = FinishJson {
        batch_pay_id: request_id.clone(),
    };
    let json_data = json!(data);

    let response = Client::new()
        .post("http://example.com")
        .body(json_data.to_string())
        .header("X-KSY-REQUEST-ID", req_uuid.clone())
        .header("X-KSY-KINGSTAR-ID", "20004")
        .send()
        .await;
    match response {
        Ok(response) => {
            let status_code = response.status();
            match response.text().await {
                Ok(body) => {
                    println!("Response status code: {}", status_code);
                    println!("Response body: {}", body);
                    return status_code.as_u16() as i32;
                }
                Err(err) => {
                    tracing::error!("Error reading response body: {}", err);
                    return 0;
                }
            }
        }
        Err(err) => {
            tracing::error!("Error sending requesst: {}", err.to_string());
            return 0;
        }
    }
}

async fn do_batch_pay(body: BatchPayJson) {
    pay_funds(body.uids).await;
    // call batch_pay_finish when all user finish
    let uuid: String = Uuid::new_v4().to_string();
    loop {
        match tokio::time::timeout(
            Duration::from_millis(600),
            batch_pay_finish(uuid.clone(), body.batch_pay_id.clone()),
        )
        .await
        {
            Ok(code) => {
                if code == 200 {
                    // print time cost
                    return;
                } else {
                    continue;
                }
            }
            // 超时重试
            Err(_) => {
                continue;
            }
        };
    }
}

async fn pay_funds(uids: Vec<i64>) {
    let mut handlers = vec![];
    for uid in uids {
        let handle = tokio::spawn(async move {
            let amount = get_all_fund(uid).await;
            if let Ok(amount) = amount {
                let start = Instant::now();
                db::api::add_money(uid, amount);
                println!(
                    "uid: {}, add money: {}, use time: {}ms",
                    uid,
                    amount,
                    start.elapsed().as_millis()
                )
            }
        });
        handlers.push(handle);
    }

    for handle in handlers {
        handle.await.unwrap(); // 等待全部完成
    }
    // println!("{:?}", db::api::get_all_balance());
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use reqwest::Client;
    use serde_json::json;
    use tokio::time::Instant;
    use uuid::Uuid;

    use crate::fund::{init_funds, Fund};

    use super::{BatchPayJson, UserTradeJson};

    #[tokio::test]
    async fn test_batch_pay() {
        let funds = vec![
            Fund {
                uid: 100001,
                amount: 88.91,
            },
            Fund {
                uid: 100042,
                amount: 10000.93,
            },
            Fund {
                uid: 403131,
                amount: 2345.35,
            },
            Fund {
                uid: 100052,
                amount: 88.93,
            },
        ];
        let res = init_funds(funds).await;
        dbg!(res.unwrap());
    }

    #[tokio::test]
    async fn test_batch_pay_once() {
        let funds = vec![Fund {
            uid: 100001,
            amount: 36.73,
        }];
        init_funds(funds.clone()).await.unwrap();
        let mut uids = vec![];
        for f in funds {
            uids.push(f.uid);
        }
        pay_funds_api(uids).await;
    }

    #[tokio::test]
    async fn test_batch_pay_from_file() {
        let funds: Vec<Fund> = match File::open("testfile/initFund100.json") {
            Ok(file) => serde_json::from_reader(file).unwrap(),
            Err(err) => {
                eprintln!("Error reading file: {}", err);
                return;
            }
        };
        let res = init_funds(funds).await;
        dbg!(res.unwrap());
    }

    #[tokio::test]
    async fn test_user_trade() {
        let funds: Vec<Fund> = match File::open("testfile/initFund100.json") {
            Ok(file) => serde_json::from_reader(file).unwrap(),
            Err(err) => {
                eprintln!("Error reading file: {}", err);
                return;
            }
        };
        init_funds(funds.clone()).await.unwrap();
        // pay all funds
        let mut uids = vec![];
        for f in funds {
            uids.push(f.uid);
        }
        pay_funds_api(uids).await;
    }

    async fn get_fund_account(uids: Vec<i64>) {
        let unique_id = Uuid::new_v4().to_string();
        let json_data = json!(uids);
        let response = Client::new()
            .post("http://127.0.0.1:20004/onePass/queryUserAmount")
            .header("Content-Type", "application/json")
            .header("X-KSY-REQUEST-ID", unique_id)
            .body(json_data.to_string())
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.text().await.unwrap();
        println!("Response status code: {}", status);
        println!("Response body: {}", body);
    }

    async fn pay_funds_api(uids: Vec<i64>) {
        let unique_id = Uuid::new_v4().to_string();
        let data = BatchPayJson {
            batch_pay_id: unique_id.clone(),
            uids,
        };
        let json_data = json!(data);
        let _response = Client::new()
            .post("http://127.0.0.1:20004/onePass/batchPay")
            .header("Content-Type", "application/json")
            .header("X-KSY-REQUEST-ID", unique_id)
            .body(json_data.to_string())
            .send()
            .await
            .unwrap();
    }

    async fn transfer_funds_to_one_account(funds: Vec<Fund>) {
        let time_start = Instant::now();
        // transfer the funds to one account
        for f in funds {
            if f.uid == 100001 {
                continue;
            }
            transfer_api(f.uid, 100001, f.amount)
                .await
                .map_err(|err| format!("Error transfering fund: {}", err.to_string()))
                .unwrap();
        }
        println!("Transfer time: {}ms", time_start.elapsed().as_millis());
    }

    async fn transfer_api(from: i64, to: i64, amount: f64) -> anyhow::Result<()> {
        let data = UserTradeJson {
            source_uid: from,
            target_uid: to,
            amount,
        };
        let json_data = json!(data);
        let unique_id = Uuid::new_v4().to_string();
        let response = Client::new()
            .post("http://127.0.0.1:20004/onePass/userTrade")
            .header("Content-Type", "application/json")
            .header("X-KSY-REQUEST-ID", unique_id)
            .body(json_data.to_string())
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.text().await.unwrap();
        println!("Response status code: {status}");
        println!("REsponse body: {body}");
        Ok(())
    }
}
