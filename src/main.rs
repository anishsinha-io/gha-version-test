mod errors;
use errors::AppError;
use rand::distributions::Alphanumeric;
use rand::Rng;

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
                        msg: "[NEW]: test service".to_string(),
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
                        msg: format!("[NEW]: current unix timestamp: {}", time),
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
                        msg: format!("[NEW]: Hello, {}", user.name),
                    }),
                )
            }),
        )
        .route(
            "/random-string",
            routing::get(|| async {
                let random_string = rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(12)
                    .map(char::from)
                    .collect::<String>();

                Json(Data {
                    msg: format!("[NEW]: random string: {}", random_string),
                })
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
            msg: format!("[NEW]: your IP address is: {}", resp["origin"]),
        }),
    )
        .into_response())
}
