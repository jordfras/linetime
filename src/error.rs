use std::fmt;

/// Custom error which holds a message and possibly an underlying error which caused it
#[derive(Debug)]
pub struct Error {
    message: String,
    cause: Option<Box<dyn std::error::Error>>,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cause: None,
        }
    }

    pub fn wrap(message: impl Into<String>, cause: Box<dyn std::error::Error>) -> Self {
        Self {
            message: message.into(),
            cause: Some(cause),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(cause) = &self.cause {
            write!(f, ": {}", cause.as_ref())?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause.as_deref()
    }
}
