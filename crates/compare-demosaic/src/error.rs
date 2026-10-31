use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorClass {
    Check,
    Invalid,
    Usage,
}

impl ErrorClass {
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Check => 1,
            Self::Invalid => 2,
            Self::Usage => 3,
        }
    }
}

#[derive(Debug)]
pub struct CompareError {
    class: ErrorClass,
    message: String,
}

impl CompareError {
    pub fn check<T>(message: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            class: ErrorClass::Check,
            message: message.into(),
        }
    }

    pub fn invalid<T>(message: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            class: ErrorClass::Invalid,
            message: message.into(),
        }
    }

    pub fn usage<T>(message: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            class: ErrorClass::Usage,
            message: message.into(),
        }
    }

    pub const fn class(&self) -> ErrorClass { self.class }

    pub fn message(&self) -> &str { &self.message }
}

impl Display for CompareError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CompareError {}

impl From<std::io::Error> for CompareError {
    fn from(error: std::io::Error) -> Self { Self::usage(error.to_string()) }
}

impl From<serde_json::Error> for CompareError {
    fn from(error: serde_json::Error) -> Self { Self::invalid(error.to_string()) }
}

pub type Result<T> = std::result::Result<T, CompareError>;
