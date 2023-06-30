pub mod config;
pub use config::Config;

pub mod request;
pub use request::Request;

pub mod response;
pub use response::{Response, ResponseError};

pub mod output;
pub use output::{List, OutputFormat};

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref VARIABLE: Regex = Regex::new(r"\$\{\s*([-.\w]+)\s*\}").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_regex() {
        let tests = vec![
            ("Hello, ${name}", vec!["name"]),
            ("Hello, ${   name  }! how are you?", vec!["name"]),
            (
                "Hello, ${ name }! How are you, ${    name}?",
                vec!["name", "name"],
            ),
            (
                "Hello, ${name }! How are you, ${name    }?",
                vec!["name", "name"],
            ),
            (
                "Hello, ${ cheese_and_toast }${toast_and_cheese}",
                vec!["cheese_and_toast", "toast_and_cheese"],
            ),
            ("howdy, ${ responses.get.name }", vec!["responses.get.name"]),
        ];

        for (input, expected) in tests {
            let mut actual = vec![];
            for capture in VARIABLE.captures_iter(input) {
                actual.push(capture.get(1).unwrap().as_str());
            }
            assert_eq!(actual, expected);
        }
    }
}
