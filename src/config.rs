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
        let server_url = env_to_string_config("SERVER_URL", "127.0.0.1".to_string());
        let port = env_to_u16_config("PORT", 3000);
        let api_prefix = env_to_string_config("API_PREFIX", "/api".to_string());
        let set_log_level = if let Some(l) = log_level {
            l
        } else {
            match env_to_log_level("LOG_LEVEL", Level::DEBUG) {
                Ok(l) => l,
                Err(e) => panic!("Error loading config: {e}"),
            }
        };
        let database_address = env_to_string_config("DATABASE_ADDRESS", "127.0.0.1".to_string());
        let database_user = env_to_string_config("DATABASE_USER", "postgres".to_string());
        let database_password = env_to_string_config_no_default("DATABASE_PASSWORD", "No database password provided. The DATABASE_PASSWORD environment vairable needs to be set");
        let database_port = env_to_u16_config("DATABASE_PORT", 5432);
        let valkey_address = env_to_string_config("VALKEY_ADDRESS", "127.0.0.1".to_string());
        let valkey_password = env_to_string_config_no_default(
            "VALKEY_PASSWORD",
            "No valkey password provided. The VALKEY_PASSWORD vairable needs to be set",
        );
        let valkey_port = env_to_u16_config("VALKEY_PORT", 6379);

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

fn env_to_string_config(env_var: &str, default: String) -> String {
    env::var(env_var).unwrap_or(default)
}

fn env_to_string_config_no_default(env_var: &str, error_msg: &str) -> String {
    env::var(env_var).expect(error_msg)
}

fn env_to_u16_config(env_var: &str, default: u16) -> u16 {
    if let Ok(port) = env::var(env_var) {
        if let Ok(p) = port.parse::<u16>() {
            p
        } else {
            default
        }
    } else {
        default
    }
}

fn env_to_log_level(env_var: &str, default: Level) -> Result<Level> {
    if let Ok(log_level) = env::var(env_var) {
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
    } else {
        Ok(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;
    use uuid::Uuid;

    #[test]
    fn env_to_string_config_from_env() {
        dotenv().ok();
        let expected = env::var("DATABASE_PASSWORD").unwrap();
        let got = env_to_string_config("DATABASE_PASSWORD", "bad".to_string());

        assert_eq!(got, expected);
    }

    #[test]
    fn env_to_string_config_default() {
        let expected = "hi";
        let got = env_to_string_config(&Uuid::new_v4().to_string(), expected.to_string());

        assert_eq!(got, expected.to_string());
    }

    #[test]
    fn env_to_u16_config_default() {
        let expected = 1111;
        let got = env_to_u16_config(&Uuid::new_v4().to_string(), expected);

        assert_eq!(got, expected);
    }

    #[test]
    fn env_to_log_level_default() {
        let expected = Level::DEBUG;
        let got = env_to_log_level(&Uuid::new_v4().to_string(), expected).unwrap();

        assert_eq!(got, expected);
    }
}
