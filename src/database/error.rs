#[derive(Debug)]
pub enum DBError {
    SQLXError(sqlx::Error),
    UnexpectedRowsAffected { expected: u64, actual: u64 },
    NoResult
}

impl From<sqlx::Error> for DBError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => DBError::NoResult,
            _ => DBError::SQLXError(err),
        }
    }
}

impl std::fmt::Display for DBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = match self {
            DBError::SQLXError(err) => err.to_string(),
            DBError::UnexpectedRowsAffected{ expected, actual } => {
                format!("Expected '{}' rows to change, saw '{}'", expected, actual)
            },
            DBError::NoResult => "A query resulted in no rows being returned".to_string()
        };
        write!(f, "{}", output)
    }
}