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

use crate::{db, fund::get_all_fund};

#[derive(Deserialize, Serialize)]
struct BatchPayJson {
    #[serde(rename = "batchPayId")]
    batch_pay_id: String,
    #[serde(rename = "uids")]
    uids: Vec<i64>,
}

pub async fn batch_pay(header: HeaderMap, body_raw: String) -> impl IntoResponse {
    match serde_json::from_str::<BatchPayJson>(&body_raw) {
        // 解析失败，返回错误信息
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid JSON"})),
        )
            .into_response(),
        Ok(body) => {
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
    }
}

pub async fn user_trade() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "code": 200
        })),
    )
}

pub async fn query_user_amount() -> impl IntoResponse {}

#[derive(Serialize)]
struct FinishJson {
    #[serde(rename = "batchPayId")]
    batch_pay_id: String,
}

pub async fn batch_pay_finish(req_uuid: String, request_id: String) -> i32 {
    let data = FinishJson {
        batch_pay_id: req_uuid.clone(),
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
    println!("{:?}", db::api::get_all_balance());

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
mod test {
    use super::batch_pay_finish;

    #[tokio::test]
    async fn test_batch_pay_finish() {
        let req_uuid = "1234567890".to_string();
        let request_id = "abcde12345".to_string();
        let code = batch_pay_finish(req_uuid.clone(), request_id.clone()).await;
        assert_eq!(code, 200);
    }
}