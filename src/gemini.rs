// Inspired by async-openai.

pub struct Client<C: Config> {
    config: C,
}

impl<C: Config> Client<C> {
    pub fn with_config(config: C) -> Self {
        Self { config }
    }
}

pub trait Config: Clone {
    fn api_key(&self) -> &str;
    fn model(&self) -> &str;
}

#[derive(Clone)]
pub struct GeminiConfig {
    api_key: String,
    model: String,
}

impl GeminiConfig {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = api_key.into();
        self
    }

    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = model.into();
        self
    }
}

impl Config for GeminiConfig {
    fn api_key(&self) -> &str {
        &self.api_key
    }
    fn model(&self) -> &str {
        &self.model
    }
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| "".to_string()),
            model: std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-pro".to_string()),
        }
    }
}
