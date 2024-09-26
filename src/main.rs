mod errors;
use async_nats::{ConnectOptions, Message};
use axum::extract::State;
use errors::AppError;
use futures::StreamExt;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::prelude::FromRow;
use sqlx::PgPool;
use tokio::{select, task};
use uuid::Uuid;

use std::env;
use std::sync::Arc;

use anyhow::Result;
use axum::http::StatusCode;

use axum::response::{IntoResponse, Response};
use axum::{routing, Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Data {
    msg: String,
}

#[derive(Serialize, Deserialize)]
struct CreateUser {
    name: String,
}

#[derive(Serialize, Deserialize, FromRow)]
struct User {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

struct AppState {
    db: PgPool,
    mailbox: Arc<tokio::sync::Mutex<Vec<Message>>>,
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
    if dotenvy::dotenv().is_ok() {
        println!("Loaded .env file");
    } else {
        println!("No .env file found");
    }
    let msgs = tokio::sync::Mutex::new(Vec::<Message>::new());

    let pg_url = env::var("DATABASE_URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(100)
        .connect(&pg_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;
    println!("Ran migrations");

    let db = pool;
    let mailbox = Arc::new(msgs);

    let app_state = AppState {
        db: db.clone(),
        mailbox: mailbox.clone(),
    };

    let (nats_token, nats_host) = (env::var("NATS_TOKEN"), env::var("NATS_HOST"));

    let client = match (nats_token, nats_host) {
        (Ok(token), Ok(host)) => {
            let url = format!("nats://{}:4222", host);
            let opts = ConnectOptions::new().token(token);
            println!("Connecting to NATS server at: {}", url);
            async_nats::connect_with_options(&url, opts).await?
        }
        (_, _) => {
            println!("Connecting to local NATS server");
            async_nats::connect("nats://localhost:4222").await?
        }
    };

    task::spawn(async move {
        match process_events(client.clone(), "asdf.asdf", mailbox.clone()).await {
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
        .route("/users", routing::get(get_users))
        .route("/users", routing::post(create_user))
        .route(
            "/events",
            routing::get(|State(state): State<Arc<AppState>>| async move {
                let x = state.mailbox.clone().lock().await.clone();

                (
                    StatusCode::OK,
                    Json(Data {
                        msg: format!("[NEW]: received {} messages", x.len()),
                    }),
                )
            }),
        )
        .with_state(Arc::new(app_state));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8181").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_ip() -> Result<Response, AppError> {
    let res = reqwest::get("https://curlmyip.org").await?;
    Ok((
        StatusCode::OK,
        Json(Data {
            msg: format!("[NEW]: your IP address is: {}", res.text().await?),
        }),
    )
        .into_response())
}

async fn get_users(State(state): State<Arc<AppState>>) -> Result<Response, AppError> {
    let pool = &state.db;

    let users = sqlx::query_as::<_, User>("select id, created_at, updated_at, name from users")
        .fetch_all(pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({"users": users }))).into_response())
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(user): Json<CreateUser>,
) -> Result<Response, AppError> {
    let pool = &state.db;
    let id = sqlx::query_scalar::<_, Uuid>("insert into users (name) values ($1) returning id")
        .bind(user.name)
        .fetch_one(pool)
        .await?;
    Ok((StatusCode::CREATED, Json(json!({"id": id}))).into_response())
}
