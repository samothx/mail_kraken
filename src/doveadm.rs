const DOVEADM_CMD: &str = "doveadm";

mod auth;
mod fetch;
pub use auth::authenticate;
pub use fetch::{
    params::{FetchParams, ImapField, SearchParam},
    Fetch, FetchFieldRes, FetchRecord,
};
