use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

pub async fn batch_pay() -> impl IntoResponse {}

pub async fn user_trade() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "code": 200
        })),
    )
}

pub async fn query_user_amount() -> impl IntoResponse {}