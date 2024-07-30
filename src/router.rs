use axum::{routing::post, Router};

use crate::handler::{batch_pay, query_user_amount, user_trade};

pub fn routers() -> Router {
    Router::new().nest(
        "/onePass",
        Router::new()
            .route("/batchPay", post(batch_pay))
            .route("/userTrade", post(user_trade))
            .route("/queryUserAmount", post(query_user_amount)),
    )
}
