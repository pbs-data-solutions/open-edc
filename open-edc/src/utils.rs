use uuid::Uuid;

pub fn generate_db_id() -> String {
    Uuid::new_v4().to_string()
}
