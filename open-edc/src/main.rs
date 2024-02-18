mod api;
mod cli;
mod db;

use std::env;

use anyhow::Result;
use axum::{serve, Router};
use clap::Parser;
use dotenvy::dotenv;

use crate::api::v1::routes::health::health_routes;
use crate::cli::{Cli, Command};
use crate::db::DbClient;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let args = Cli::parse();

    match args.command {
        Command::Start { url, port } => {
            let app = app().await;
            let server_url = url.unwrap_or("0.0.0.0".to_string());
            let server_port = port.unwrap_or(3000);
            let listener = tokio::net::TcpListener::bind(format!("{server_url}:{server_port}"))
                .await
                .unwrap();

            serve(listener, app).await.unwrap();
        }
    }

    Ok(())
}

async fn app() -> Router {
    let database_address = env::var("DATABASE_ADDRESS").unwrap_or("127.0.0.1".to_string());
    let database_user = env::var("DATASE_USER").unwrap_or("postgres".to_string());
    let database_user_password =
        env::var("DATASE_USER_PASSWORD").unwrap_or("test_password".to_string());
    let database_port = env::var("DATABASE_PORT")
        .unwrap_or("5432".to_string())
        .parse::<u16>()
        .unwrap_or(5432);
    let db_client = DbClient::new(
        &database_address,
        &database_user,
        &database_user_password,
        &database_port,
        "open_edc",
    )
    .await
    .expect("Unable to connect to database");
    let pool = db_client
        .create_pool(None, None)
        .await
        .expect("Unable to connect to the database");

    Router::new()
        .merge(health_routes(pool.clone()))
        .with_state(pool)
}
