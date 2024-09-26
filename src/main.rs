mod errors;
use async_nats::Message;
use errors::AppError;
use futures::StreamExt;
use rand::distributions::Alphanumeric;
use rand::Rng;
use tokio::{select, task};

use std::collections::HashMap;
use std::env;
use std::sync::Arc;

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

async fn process_events(
    client: async_nats::Client,
    topic: &str,
    msgs: Arc<tokio::sync::Mutex<Vec<Message>>>,
) -> Result<()> {
    let mut sub = client.subscribe(topic.to_string()).await?;
    loop {
        select! {
            res = sub.next() => {
                if let Some(msg) = res {
                    println!("Received message: {:?}", msg);
                    msgs.lock().await.push(msg);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let msgs = Arc::new(tokio::sync::Mutex::new(Vec::<Message>::new()));

    let (nats_token, nats_host) = (env::var("NATS_TOKEN"), env::var("NATS_HOST"));
    let nats_url = match (nats_token, nats_host) {
        (Ok(token), Ok(host)) => format!("{}@{}:4222", token, host),
        (_, _) => "nats://localhost:4222".to_string(),
    };

    println!("Connecting to NATS server at: {}", nats_url);

    let client = async_nats::connect(&nats_url).await?;

    let msgs_copy = msgs.clone();

    task::spawn(async move {
        match process_events(client.clone(), "asdf.asdf", msgs.clone()).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
    });

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
        .route("/ip", routing::get(get_ip))
        .route(
            "/events",
            routing::get(|| async move {
                let x = msgs_copy.clone().lock().await.clone();

                (
                    StatusCode::OK,
                    Json(Data {
                        msg: format!("[NEW]: received {} messages", x.len()),
                    }),
                )
            }),
        );

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
