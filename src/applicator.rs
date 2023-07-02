use std::collections::HashMap;

use crate::Response;

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref VARIABLE: Regex = Regex::new(r"\$\{\s*([-.\w]+)\s*\}").unwrap();
}

#[derive(Default)]
pub struct Applicator {
    context: HashMap<String, String>,
    responses: HashMap<String, Response>,
}

impl Applicator {
    pub fn new(context: HashMap<String, String>, responses: HashMap<String, Response>) -> Self {
        Self { context, responses }
    }

    pub fn add_response(&mut self, name: String, response: Response) {
        self.responses.insert(name, response);
    }

    pub fn apply(&self, s: &str) -> String {
        let mut output = String::new();
        let mut last = 0;

        for capture in VARIABLE.captures_iter(s) {
            let r = capture.get(0).unwrap().range();
            let name = capture.get(1).unwrap().as_str();
            output.push_str(&s[last..r.start]);
            let replacement = match name.starts_with("response.") {
                true => match self.find_response_data(&name[9..]) {
                    Some(v) => v,
                    None => "".to_string(),
                },
                false => match self.context.get(name) {
                    Some(v) => v.clone(),
                    None => "".to_string(),
                },
            };

            output.push_str(&replacement);

            last = r.end;
        }

        output.push_str(&s[last..]);
        output
    }

    fn find_response_data(&self, name: &str) -> Option<String> {
        // Split the request name and the path.
        let tokens = name.splitn(2, '.').collect::<Vec<_>>();
        if tokens.len() != 2 {
            return None;
        }
        // Get the response and try to find the path.
        let response = self.responses.get(tokens[0])?;
        response.find_path_in_body(tokens[1])
    }
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

    #[test]
    fn test_apply() {
        use crate::Response;

        let mut context = HashMap::new();
        context.insert("name".to_string(), "World".to_string());
        context.insert("age".to_string(), "4.543 Billion".to_string());

        let mut responses = HashMap::new();
        responses.insert(
            "hello".to_string(),
            Response {
                status_code: 200,
                version: "HTTP/1.1".to_string(),
                headers: HashMap::new(),
                body: "{ \"name\": \"Galaxy\", \"age\": \"13.61 Billion\" }".to_string(),
            },
        );

        let app = Applicator { context, responses };

        let s = app.apply("Hello, ${name}! You are ${age} years old. My name is ${response.hello.name}. I am ${response.hello.age} years old.${response.hello.some.bad.one}${response.}");
        assert_eq!(
            s,
            "Hello, World! You are 4.543 Billion years old. My name is Galaxy. I am 13.61 Billion years old."
        );
    }
}
