use std::{collections::HashMap, sync::Arc};

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Mutex;

use crate::fund::get_all_fund;

#[derive(Deserialize, Serialize)]
struct BatchPayJson {
    #[serde(rename = "batchPayId")]
    batch_pay_id: String,
    #[serde(rename = "uids")]
    uids: Vec<i64>,
}

pub async fn batch_pay(body_raw: String) -> impl IntoResponse {
    match serde_json::from_str::<BatchPayJson>(&body_raw) {
        // 解析失败，返回错误信息
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid JSON"})),
        ).into_response(),
        Ok(body) => {
            // TODO: make sure a BatchPayId will only do once
            let mp = Arc::new(Mutex::new(HashMap::new()));
            let mut handlers = vec![];
            for uid in body.uids {
                let mp = Arc::clone(&mp);
                let handle = tokio::spawn(async move {
                    let amount = get_all_fund(uid).await;
                    if let Ok(amount) = amount {
                        mp.lock().await.insert(uid, amount);
                    }
                });
                handlers.push(handle);
            }
            
            for handle in handlers {
                handle.await.unwrap(); // 等待全部完成
            }
            println!("{:?}", mp.lock().await);
            ().into_response()
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
