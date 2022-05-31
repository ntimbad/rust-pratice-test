use futures::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    // Errors that should stop the program
    #[error("Recoverable Error {0}")]
    Recoverable(RuntimeErrorType),

    #[error("NonRecoverable Error {0}")]
    NonRecoverable(RuntimeErrorType),
}

#[derive(Error, Debug)]
pub enum RuntimeErrorType {
    #[error("Parse Error {0}")]
    ParseError(String),
    #[error("CSVFileReadError {0}")]
    CSVFileReadWriteError(String),
    #[error("CSVFileReadError {0}")]
    CSVLineParseError(String),
    #[error("Transaction file ops {0}")]
    TransactionFileOps(String),
    #[error("BalanceIssues {0}")]
    BalanceIssues(String),
    #[error("Locked Account {0}")]
    LockedAccount(String),
    #[error("WrongTransactionState {0}")]
    WrongTransactionState(String),
    #[error("Join error {0}")]
    JoinError(String),
    #[error("TransactionAlreadyPresent")]
    TransactionAlreadyPresent,
    #[error(transparent)]
    IOError(#[from] io::Error),
}
