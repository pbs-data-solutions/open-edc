use std::sync::Arc;

use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use tokio::task::spawn_blocking;
use uuid::Uuid;

pub fn generate_db_id() -> String {
    Uuid::new_v4().to_string()
}

pub async fn hash_password(password: &str) -> Result<String> {
    let password_arc = Arc::new(password.to_string());

    let hashed_password = spawn_blocking(move || -> Result<String> {
        let password = password_arc.clone();
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hashed_password = argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string();

        Ok(hashed_password)
    })
    .await??;

    Ok(hashed_password)
}

#[allow(dead_code)]
pub async fn verify_password(password: &str, hashed_password: &str) -> Result<()> {
    let password_arc = Arc::new(password.to_string());
    let password_hash_arc = Arc::new(hashed_password.to_string());

    spawn_blocking(move || -> Result<()> {
        let password = password_arc.clone();
        let hashed_password = password_hash_arc.clone();
        let parsed_hash = PasswordHash::new(&hashed_password)?;
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash)?;

        Ok(())
    })
    .await??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_password() {
        let password = "some_password".to_string();
        let hashed_password = hash_password(&password).await.unwrap();
        assert_ne!(password, hashed_password);
    }

    #[tokio::test]
    async fn test_verify_password() {
        let password = "some_password".to_string();
        let hashed_password = hash_password(&password).await.unwrap();
        assert!(verify_password(&password, &hashed_password).await.is_ok());
    }
}
