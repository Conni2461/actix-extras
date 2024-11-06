use derive_more::derive::{Display, Error, From};

use crate::status::Status;

/// Failure modes of the rate limiter.
#[derive(Debug, Display, Error, From)]
pub enum Error {
    /// Failed to Track connection.
    #[display("Failed to Track connection")]
    #[from(ignore)]
    Track(#[error(not(source))] String),

    /// Limit is exceeded for a key.
    #[display("Limit is exceeded for a key")]
    #[from(ignore)]
    LimitExceeded(#[error(not(source))] Status),

    /// Time conversion failed.
    #[display("Time conversion failed")]
    Time(time::error::ComponentRange),

    /// Generic error.
    #[display("Generic error")]
    #[from(ignore)]
    Other(#[error(not(source))] String),
}

#[cfg(test)]
mod tests {
    use super::*;

    static_assertions::assert_impl_all! {
        Error:
        From<time::error::ComponentRange>,
    }

    static_assertions::assert_not_impl_any! {
        Error:
        From<String>,
        From<Status>,
    }
}
