//! The practice set: its format, its validation, and lookups.
//!
//! A set is multiple choice. The answer key of a question is the text of its
//! correct choice, which must be exactly one of the choices.

use serde::{Deserialize, Serialize};

/// Format tag every practice set must carry.
pub const SET_FORMAT: &str = "practice-set/1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Topic {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub id: String,
    pub topic: String,
    pub prompt: String,
    pub choices: Vec<String>,
    pub answer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PracticeSet {
    pub format: String,
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub sample: bool,
    #[serde(default)]
    pub note: String,
    pub topics: Vec<Topic>,
    pub questions: Vec<Question>,
}

impl PracticeSet {
    /// Display name of a topic id ("" if unknown; a validated set has none).
    pub fn topic_name(&self, topic_id: &str) -> &str {
        self.topics
            .iter()
            .find(|t| t.id == topic_id)
            .map(|t| t.name.as_str())
            .unwrap_or("")
    }

    /// Position of a question id in the set.
    pub fn question_index(&self, question_id: &str) -> Option<usize> {
        self.questions.iter().position(|q| q.id == question_id)
    }

    /// Position of the correct choice of question `q` (validated to exist).
    pub fn answer_index(&self, q: usize) -> Option<usize> {
        let question = self.questions.get(q)?;
        question.choices.iter().position(|c| *c == question.answer)
    }
}

/// An id may hold ASCII letters, digits, `-` and `_` only, so it is safe in
/// file names and in the canonical text that the check value covers.
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Reads and validates a practice set.
pub fn parse_set(text: &str) -> Result<PracticeSet, String> {
    let set: PracticeSet = serde_json::from_str(text)
        .map_err(|e| format!("The practice set is not valid: {e}."))?;
    validate_set(&set)?;
    Ok(set)
}

fn validate_set(set: &PracticeSet) -> Result<(), String> {
    if set.format != SET_FORMAT {
        return Err(format!(
            "The practice set has format \"{}\"; this tool reads \"{}\".",
            set.format, SET_FORMAT
        ));
    }
    if !is_valid_id(&set.id) {
        return Err(format!(
            "The practice set id \"{}\" must be 1 to 64 letters, digits, '-' or '_'.",
            set.id
        ));
    }
    if set.title.trim().is_empty() {
        return Err("The practice set has no title.".to_string());
    }
    if set.topics.is_empty() {
        return Err("The practice set has no topics.".to_string());
    }
    if set.questions.is_empty() {
        return Err("The practice set has no questions.".to_string());
    }
    for (i, t) in set.topics.iter().enumerate() {
        if !is_valid_id(&t.id) {
            return Err(format!(
                "Topic id \"{}\" must be 1 to 64 letters, digits, '-' or '_'.",
                t.id
            ));
        }
        if t.name.trim().is_empty() {
            return Err(format!("Topic \"{}\" has no name.", t.id));
        }
        if set.topics[..i].iter().any(|u| u.id == t.id) {
            return Err(format!("Topic \"{}\" appears more than once.", t.id));
        }
    }
    for (i, q) in set.questions.iter().enumerate() {
        if !is_valid_id(&q.id) {
            return Err(format!(
                "Question id \"{}\" must be 1 to 64 letters, digits, '-' or '_'.",
                q.id
            ));
        }
        if set.questions[..i].iter().any(|r| r.id == q.id) {
            return Err(format!("Question \"{}\" appears more than once.", q.id));
        }
        if !set.topics.iter().any(|t| t.id == q.topic) {
            return Err(format!(
                "Question \"{}\" has topic \"{}\", which is not in the topic list.",
                q.id, q.topic
            ));
        }
        if q.prompt.trim().is_empty() {
            return Err(format!("Question \"{}\" has no prompt.", q.id));
        }
        if q.choices.len() < 2 {
            return Err(format!("Question \"{}\" needs at least two choices.", q.id));
        }
        for (j, c) in q.choices.iter().enumerate() {
            if c.trim().is_empty() {
                return Err(format!("Question \"{}\" has an empty choice.", q.id));
            }
            if q.choices[..j].contains(c) {
                return Err(format!(
                    "Question \"{}\" has the choice \"{}\" more than once.",
                    q.id, c
                ));
            }
        }
        if !q.choices.contains(&q.answer) {
            return Err(format!(
                "The answer of question \"{}\" (\"{}\") is not one of its choices.",
                q.id, q.answer
            ));
        }
    }
    for t in &set.topics {
        if !set.questions.iter().any(|q| q.topic == t.id) {
            return Err(format!("Topic \"{}\" has no questions.", t.id));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const SAMPLE: &str = include_str!("../../../data/practice-set.json");

    pub(crate) fn sample_set() -> PracticeSet {
        parse_set(SAMPLE).expect("sample set is valid")
    }

    fn unchecked_sample() -> PracticeSet {
        serde_json::from_str(SAMPLE).expect("sample set parses")
    }

    fn rejected(set: &PracticeSet) -> String {
        let text = serde_json::to_string(set).expect("serialise");
        parse_set(&text).expect_err("this set should be rejected")
    }

    #[test]
    fn sample_set_has_12_questions_in_4_topics() {
        let set = sample_set();
        assert_eq!(set.questions.len(), 12);
        assert_eq!(set.topics.len(), 4);
        for t in &set.topics {
            let n = set.questions.iter().filter(|q| q.topic == t.id).count();
            assert_eq!(n, 3, "topic {} should have 3 questions", t.id);
        }
        for (i, _) in set.questions.iter().enumerate() {
            assert!(set.answer_index(i).is_some());
        }
    }

    #[test]
    fn sample_set_is_marked_as_sample() {
        let set = sample_set();
        assert!(set.sample);
        assert!(set.title.contains("SAMPLE"), "title: {}", set.title);
        assert!(!set.note.trim().is_empty());
    }

    #[test]
    fn rejects_malformed_json() {
        let err = parse_set("{ \"format\": \"practice-set/1\", ").unwrap_err();
        assert!(err.starts_with("The practice set is not valid"), "{err}");
    }

    #[test]
    fn rejects_unknown_field() {
        let text = SAMPLE.replacen("\"sample\": true", "\"sample\": true, \"extra\": 1", 1);
        let err = parse_set(&text).unwrap_err();
        assert!(err.contains("unknown field `extra`"), "{err}");
    }

    #[test]
    fn rejects_answer_not_in_choices() {
        let mut set = unchecked_sample();
        set.questions[0].answer = "7 : 9".to_string();
        let err = rejected(&set);
        assert!(err.contains("q01") && err.contains("not one of its choices"), "{err}");
    }

    #[test]
    fn rejects_unknown_topic() {
        let mut set = unchecked_sample();
        set.questions[0].topic = "fractions".to_string();
        let err = rejected(&set);
        assert!(err.contains("q01") && err.contains("fractions"), "{err}");
    }

    #[test]
    fn rejects_duplicate_question_id() {
        let mut set = unchecked_sample();
        set.questions[1].id = "q01".to_string();
        let err = rejected(&set);
        assert_eq!(err, "Question \"q01\" appears more than once.");
    }

    #[test]
    fn rejects_topic_without_questions() {
        let mut set = unchecked_sample();
        set.topics.push(Topic { id: "percent".into(), name: "Percentages".into() });
        let err = rejected(&set);
        assert_eq!(err, "Topic \"percent\" has no questions.");
    }

    #[test]
    fn rejects_id_unsafe_for_file_names() {
        let mut set = unchecked_sample();
        set.id = "ratios/../x".to_string();
        let err = rejected(&set);
        assert!(err.contains("ratios/../x"), "{err}");
    }
}
