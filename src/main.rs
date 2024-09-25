mod errors;
use errors::AppError;

use std::collections::HashMap;

use anyhow::Result;
use axum::http::StatusCode;

use axum::response::{IntoResponse, Response};
use axum::{routing, Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Data {
    msg: String,
}

#[derive(Serialize, Deserialize)]
struct User {
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = Router::new()
        .route(
            "/",
            routing::get(|| async {
                (
                    StatusCode::OK,
                    Json(Data {
                        msg: "Test service".to_string(),
                    }),
                )
            }),
        )
        .route(
            "/time",
            routing::get(|| async {
                let time = Utc::now().timestamp();
                (
                    StatusCode::OK,
                    Json(Data {
                        msg: format!("Current unix timestamp: {}", time),
                    }),
                )
            }),
        )
        .route(
            "/hello",
            routing::post(|Json(user): Json<User>| async move {
                (
                    StatusCode::CREATED,
                    Json(Data {
                        msg: format!("Hello, {}", user.name),
                    }),
                )
            }),
        )
        .route("/ip", routing::get(get_ip));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8181").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_ip() -> Result<Response, AppError> {
    let resp = reqwest::get("https://httpbin.org/ip")
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    Ok((
        StatusCode::OK,
        Json(Data {
            msg: format!("Your IP address is: {}", resp["origin"]),
        }),
    )
        .into_response())
}
