use serde::Serialize;

pub mod config;
pub mod context;
pub mod request;

pub trait List: Serialize + IntoIterator<Item = Vec<String>> {
    fn headers(&self) -> Vec<String>;
}
