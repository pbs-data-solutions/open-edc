use std::env;

use anyhow::{bail, Result};
use tracing::Level;

pub struct Config {
    pub server_url: String,
    pub port: u16,
    pub api_prefix: String,
    pub log_level: Level,
    pub database_address: String,
    pub database_user: String,
    pub database_password: String,
    pub database_port: u16,
    pub valkey_address: String,
    pub valkey_password: String,
    pub valkey_port: u16,
}

impl Config {
    pub fn new(log_level: Option<Level>) -> Self {
        let server_url = env::var("SERVER_URL").unwrap_or("127.0.0.1".to_string());
        let port = env::var("PORT")
            .unwrap_or("3000".to_string())
            .parse::<u16>()
            .unwrap_or(3000);
        let api_prefix = env::var("API_PREFIX").unwrap_or("/api".to_string());
        let set_log_level = if let Some(l) = log_level {
            l
        } else {
            let string_level = env::var("LOG_LEVEL").unwrap_or("DEBUG".to_string());
            match str_to_log_level(&string_level) {
                Ok(l) => l,
                Err(e) => panic!("Error loading config: {e}"),
            }
        };
        let database_address = env::var("DATABASE_ADDRESS").unwrap_or("127.0.0.1".to_string());
        let database_user = env::var("DATABASE_USER").unwrap_or("postgres".to_string());
        let database_password = env::var("DATABASE_PASSWORD")
            .expect("No database password provided. The DATABASE_PASSWORD environment vairable needs to be set");
        let database_port = env::var("DATABASE_PORT")
            .unwrap_or("5432".to_string())
            .parse::<u16>()
            .unwrap_or(5432);
        let valkey_address = env::var("VALKEY_ADDRESS").unwrap_or("127.0.0.1".to_string());
        let valkey_password = env::var("VALKEY_PASSWORD")
            .expect("No valkey password provided. The VALKEY_PASSWORD vairable needs to be set");
        let valkey_port = env::var("VALKEY_PORT")
            .unwrap_or("6379".to_string())
            .parse::<u16>()
            .unwrap_or(6379);

        Self {
            server_url,
            port,
            api_prefix,
            log_level: set_log_level,
            database_address,
            database_user,
            database_password,
            database_port,
            valkey_address,
            valkey_password,
            valkey_port,
        }
    }
}

fn str_to_log_level(log_level: &str) -> Result<Level> {
    let upper_log_level = log_level.to_uppercase();

    if upper_log_level == "TRACE" {
        Ok(Level::TRACE)
    } else if upper_log_level == "DEBUG" {
        Ok(Level::DEBUG)
    } else if upper_log_level == "Warn" {
        Ok(Level::WARN)
    } else if upper_log_level == "INFO" {
        Ok(Level::DEBUG)
    } else if upper_log_level == "Error" {
        Ok(Level::ERROR)
    } else {
        bail!("{log_level} is not a valid log level");
    }
}
