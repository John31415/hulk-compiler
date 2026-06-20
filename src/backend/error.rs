use std::fmt;

#[derive(Debug)]
pub enum BackendError {
    UnknownType(String),
    UnknownFunction(String),
    UndefinedVariable,
    Io(std::io::Error),
    InvalidExpression,
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::UnknownType(t) => write!(f, "unknown type: {}", t),
            BackendError::UnknownFunction(name) => write!(f, "unknown function: {}", name),
            BackendError::UndefinedVariable => write!(f, "undefined variable"),
            BackendError::InvalidExpression => write!(f, "invalid expression"),
            BackendError::Io(err) => write!(f, "io error: {}", err),
        }
    }
}

impl std::error::Error for BackendError {}

impl From<std::io::Error> for BackendError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub type BackendResult<T> = Result<T, BackendError>;
