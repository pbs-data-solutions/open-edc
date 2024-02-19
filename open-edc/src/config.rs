pub struct Config {
    pub prefix: String,
}

impl Config {
    pub fn new(prefix: Option<String>) -> Self {
        Self {
            prefix: prefix.unwrap_or("/api/v1".to_string()),
        }
    }
}
