use axum::extract::FromRef;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use sqlx::postgres::PgPool;

#[derive(Clone)]
pub struct DbState {
    pub pool: PgPool,
}

impl FromRef<AppState> for DbState {
    fn from_ref(app_state: &AppState) -> DbState {
        app_state.db_state.clone()
    }
}

#[derive(Clone)]
pub struct ValkeyState {
    pub pool: Pool<RedisConnectionManager>,
}

impl FromRef<AppState> for ValkeyState {
    fn from_ref(app_state: &AppState) -> ValkeyState {
        app_state.valkey_state.clone()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db_state: DbState,
    pub valkey_state: ValkeyState,
}
