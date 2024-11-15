use std::fmt;

/// Custom error which holds a context message and possibly an underlying error which caused it
#[derive(Debug)]
pub struct ErrorWithContext {
    context: String,
    cause: Box<dyn std::error::Error + Send>,
}

impl ErrorWithContext {
    pub fn wrap<E: std::error::Error + Send + 'static>(
        context: impl Into<String>,
        cause: E,
    ) -> Self {
        Self {
            context: context.into(),
            cause: Box::new(cause),
        }
    }
}

impl fmt::Display for ErrorWithContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.context, self.cause)?;
        Ok(())
    }
}

impl std::error::Error for ErrorWithContext {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

/// Extension trait to `Result` to wrap any contained error in an `ErrorWithContext`
pub trait ResultExt<T> {
    fn error_context(self, context: impl Into<String>) -> Result<T, ErrorWithContext>;
}

/// Implement `ResultExt` for any error implementing `std::error::Error` trait
impl<T, E: std::error::Error + Send + 'static> ResultExt<T> for Result<T, E> {
    fn error_context(self, context: impl Into<String>) -> Result<T, ErrorWithContext> {
        self.map_err(|error| ErrorWithContext::wrap(context, error))
    }
}
