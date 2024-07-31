use std::time::Duration;

use axum::{
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::task;
use uuid::Uuid;

use crate::{db, fund::{get_all_fund, Fund}};

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

#[derive(Deserialize)]
struct UserTradeJson {
    #[serde(rename = "sourceUid")]
    source_uid: i64,
    #[serde(rename = "targetUid")]
    target_uid: i64,
    amount: f64,
}

pub async fn batch_pay(header: HeaderMap, Json(body): Json<BatchPayJson>) -> impl IntoResponse {
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
    let body: UserTradeJson = match serde_json::from_str(&body_raw) {
        Ok(body) => body,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid JSON"
                })),
            )
        }
    };

    if let Err(err) = db::api::transfer(
        body.source_uid,
        body.target_uid,
        (body.amount * 100.0) as i64,
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": err.to_string()})),
        );
    }

    let request_id = match header.get("X-KSY-REQUEST-ID") {
        Some(value) => value.to_str().unwrap().to_string(),
        None => "".to_string(),
    };

    (
        StatusCode::OK,
        Json(json!({"msg": "ok", "code": 200, "requestId": request_id})),
    )
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
    // TODO: make sure a BatchPayId will only do once
    let mut handlers = vec![];
    for uid in body.uids {
        let handle = tokio::spawn(async move {
            let amount = get_all_fund(uid).await;
            if let Ok(amount) = amount {
                db::api::add_money(uid, amount);
            }
        });
        handlers.push(handle);
    }

    for handle in handlers {
        handle.await.unwrap(); // 等待全部完成
    }
    // println!("{:?}", db::api::get_all_balance());

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

#[cfg(test)]
mod tests {
    use std::fs::File;

    use crate::fund::{init_funds, Fund};

    
    #[tokio::test]
    async fn test_batch_pay() {
        let funds = vec![Fund{
            uid: 100032,
            amount: 88.91,
        }, Fund {
            uid: 100042,
            amount: 10000.93,
        }, Fund {
            uid:    403131,
			amount: 2345.35,
        }, Fund {
            uid:    100052,
			amount: 88.93,
        }];
        let res = init_funds(funds).await;
        dbg!(res.unwrap());
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
}