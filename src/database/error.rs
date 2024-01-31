#[derive(Debug)]
pub enum DBError {
    SQLXError(sqlx::Error),
    UnexpectedRowsAffected(u64, u64),
}

impl From<sqlx::Error> for DBError {
    fn from(err: sqlx::Error) -> Self {
        DBError::SQLXError(err)
    }
}

impl std::fmt::Display for DBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = match self {
            DBError::SQLXError(err) => err.to_string(),
            DBError::UnexpectedRowsAffected(expected, actual) => {
                format!("Expected '{}' rows to change, saw '{}'", expected, actual)
            },
        };
        write!(f, "{}", output)
    }
}