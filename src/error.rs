use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    StoatError(stoat::Error),
    DatabaseError(String),
    PoolError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::StoatError(e) => write!(f, "Stoat error: {:?}", e),
            Error::DatabaseError(e) => write!(f, "Database error: {}", e),
            Error::PoolError(e) => write!(f, "Pool error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<stoat::Error> for Error {
    fn from(value: stoat::Error) -> Self {
        Self::StoatError(value)
    }
}

impl From<r2d2::Error> for Error {
    fn from(value: r2d2::Error) -> Self {
        Self::PoolError(value.to_string())
    }
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Self::DatabaseError(value.to_string())
    }
}
