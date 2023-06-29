use std::collections::HashMap;

use serde::Serialize;

use crate::List;

pub trait Context: Serialize {
    fn apply(&self, s: &str) -> String;
    fn merge(&mut self, other: &Self);
}

impl<C: Context> List for HashMap<String, C> {
    fn headers(&self) -> Vec<String> {
        vec!["Name".into()]
    }

    fn values(&self) -> Vec<Vec<String>> {
        self.iter().map(|(n, _)| vec![n.clone()]).collect()
    }
}

impl Context for HashMap<String, String> {
    fn apply(&self, s: &str) -> String {
        let mut output = String::new();
        let mut last = 0;

        for capture in crate::VARIABLE.captures_iter(s) {
            let r = capture.get(0).unwrap().range();
            let name = capture.get(1).unwrap().as_str();

            output.push_str(&s[last..r.start]);
            output.push_str(self.get(name).unwrap_or(&"".to_string()));

            last = r.end;
        }

        output.push_str(&s[last..]);
        output
    }
    fn merge(&mut self, other: &Self) {
        self.extend(other.clone());
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

        let input_string = "My name is ${name} and I am ${age} years old.";
        let expected_string = "My name is John and I am 30 years old.";

        assert_eq!(context.apply(input_string), expected_string);
    }
}
