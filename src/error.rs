use thiserror::Error;

#[derive(Error, Debug)]
pub enum CourierError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Source fetch error: {message}")]
    SourceFetch { origin: String, message: String },

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Channel send error: {message}")]
    ChannelSend { channel: String, message: String },

    #[error("Scheduler error: {0}")]
    #[allow(dead_code)]
    Scheduler(String),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, CourierError>;
