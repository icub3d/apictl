use serde::Serialize;

pub mod config;
pub use config::Config;

pub mod context;
pub use context::Context;

pub mod request;
pub use request::Request;

pub mod response;
pub use response::Response;

pub trait List: Serialize {
    fn headers(&self) -> Vec<String>;
    fn values(&self) -> Vec<Vec<String>>;
}
