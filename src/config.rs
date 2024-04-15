pub struct Config {
    pub api_v1_prefix: String,
}

impl Config {
    pub fn new(prefix: Option<String>) -> Self {
        Self {
            api_v1_prefix: prefix.unwrap_or("/api".to_string()),
        }
    }
}
