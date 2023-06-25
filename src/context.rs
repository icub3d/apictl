use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::List;

#[derive(Debug, Serialize, Deserialize)]
pub struct Contexts {
    #[serde(flatten)]
    pub contexts: HashMap<String, Context>,
}

impl Default for Contexts {
    fn default() -> Self {
        Self::new()
    }
}

impl Contexts {
    pub fn new() -> Self {
        Self {
            contexts: HashMap::new(),
        }
    }

    pub fn merge(&mut self, other: Contexts) {
        for (k, v) in other.contexts {
            self.contexts.insert(k, v);
        }
    }
}

impl List for Contexts {
    fn headers(&self) -> Vec<String> {
        vec!["Name".into()]
    }
}

impl IntoIterator for Contexts {
    type Item = Vec<String>;
    type IntoIter = std::iter::Map<
        std::collections::hash_map::IntoIter<String, Context>,
        fn((String, Context)) -> Vec<String>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.contexts.into_iter().map(|(k, _)| vec![k])
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Context {
    #[serde(flatten)]
    context: HashMap<String, String>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            context: HashMap::new(),
        }
    }

    pub fn merge(&mut self, other: &Context) {
        for (k, v) in &other.context {
            self.context.insert(k.clone(), v.clone());
        }
    }

    pub fn apply(&self, input: &str) -> String {
        let mut output = input.to_string();

        for (k, v) in &self.context {
            output = output.replace(&format!("{{{{{}}}}}", k), v);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply() {
        let mut context = HashMap::new();
        context.insert(String::from("name"), String::from("John"));
        context.insert(String::from("age"), String::from("30"));

        let input_string = "My name is {{name}} and I am {{age}} years old.";
        let expected_string = "My name is John and I am 30 years old.";

        assert_eq!(context.apply(input_string), expected_string);
    }
}
