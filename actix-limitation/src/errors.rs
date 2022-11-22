use derive_more::{Display, Error, From};

use crate::status::Status;

/// Failure modes of the rate limiter.
#[derive(Debug, Display, Error, From)]
pub enum Error {
    /// Failed to Track connection.
    #[display(fmt = "Failed to Track connection")]
    Track(#[error(not(source))] String),

    /// Limit is exceeded for a key.
    #[display(fmt = "Limit is exceeded for a key")]
    #[from(ignore)]
    LimitExceeded(#[error(not(source))] Status),

    /// Time conversion failed.
    #[display(fmt = "Time conversion failed")]
    Time(time::error::ComponentRange),

    /// Generic error.
    #[display(fmt = "Generic error")]
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
