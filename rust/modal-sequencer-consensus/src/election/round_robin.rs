use serde::Deserialize;
use anyhow::Result;

#[derive(Deserialize)]
struct Input {
    round: u64,
}

#[derive(Clone)]
pub struct RoundRobin {}

impl RoundRobin {
    pub fn new() -> Self {
        RoundRobin {}
    }

    pub fn create() -> Self {
        Self::new()
    }

    pub async fn pick_one<'a, T>(&'a self, options: &'a [T], input: &str) -> Result<&'a T> {
        let parsed: Input = serde_json::from_str(input)?;
        // Subtract 1 from round as the first round is 1
        let i = (parsed.round - 1) as usize % options.len();
        Ok(&options[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_round_robin() {
        let choices = vec!["a", "b", "c"];
        let picker = RoundRobin::create();

        // Test round 1 (should pick first element)
        let input = r#"{"round": 1}"#;
        assert_eq!(*picker.pick_one(&choices, input).await.unwrap(), "a");

        // Test round 2 (should pick second element)
        let input = r#"{"round": 2}"#;
        assert_eq!(*picker.pick_one(&choices, input).await.unwrap(), "b");

        // Test round 4 (should wrap around to first element)
        let input = r#"{"round": 4}"#;
        assert_eq!(*picker.pick_one(&choices, input).await.unwrap(), "a");

        // Test invalid JSON
        let input = r#"{"round": invalid}"#;
        assert!(picker.pick_one(&choices, input).await.is_err());
    }
}