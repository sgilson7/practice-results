//! The result file a student saves and a teacher opens.
//!
//! It holds only: a format tag, the set id, the nickname the student chose,
//! one entry per question (question id, topic, correct), and a check value.
//! The check value is FNV-1a 64 over a canonical text of that content. It
//! catches hand edits and damaged files; it is NOT a security seal, because
//! anyone can recompute it.

use serde::{Deserialize, Serialize};

use crate::set::PracticeSet;

/// Format tag every result file carries.
pub const RESULT_FORMAT: &str = "practice-result/1";
/// Longest nickname, in characters.
pub const NICKNAME_MAX_CHARS: usize = 24;
/// Largest result file the teacher view will read, in bytes.
pub const MAX_RESULT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub question: String,
    pub topic: String,
    pub correct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultFile {
    pub format: String,
    pub set_id: String,
    pub nickname: String,
    pub entries: Vec<Entry>,
    pub check: String,
}

/// A file ready for the page to offer as a download.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedFile {
    pub file_name: String,
    pub contents: String,
}

/// FNV-1a, 64-bit.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The text the check value covers. One line each for the format, the set,
/// the nickname, then one `question|topic|1-or-0` line per entry, in order.
pub fn canonical_text(format: &str, set_id: &str, nickname: &str, entries: &[Entry]) -> String {
    let mut text = String::new();
    text.push_str(format);
    text.push('\n');
    text.push_str("set:");
    text.push_str(set_id);
    text.push('\n');
    text.push_str("nickname:");
    text.push_str(nickname);
    text.push('\n');
    for e in entries {
        text.push_str(&e.question);
        text.push('|');
        text.push_str(&e.topic);
        text.push('|');
        text.push(if e.correct { '1' } else { '0' });
        text.push('\n');
    }
    text
}

/// The check value for a result's content, e.g. `fnv1a64:0123456789abcdef`.
pub fn check_value(format: &str, set_id: &str, nickname: &str, entries: &[Entry]) -> String {
    let text = canonical_text(format, set_id, nickname, entries);
    format!("fnv1a64:{:016x}", fnv1a64(text.as_bytes()))
}

/// Cleans a nickname the student typed: trimmed, 1 to 24 characters, letters,
/// digits, spaces, `-`, `_` and `.` only.
pub fn clean_nickname(raw: &str) -> Result<String, String> {
    let nickname = raw.trim();
    if nickname.is_empty() {
        return Err("Type a nickname first (not your real name).".to_string());
    }
    let count = nickname.chars().count();
    if count > NICKNAME_MAX_CHARS {
        return Err(format!(
            "A nickname can have at most {NICKNAME_MAX_CHARS} characters (this one has {count})."
        ));
    }
    if let Some(bad) = nickname
        .chars()
        .find(|c| !(c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.')))
    {
        return Err(format!(
            "A nickname can use letters, digits, spaces, '-', '_' and '.' only (not {:?}).",
            bad
        ));
    }
    Ok(nickname.to_string())
}

/// File name offered for a saved result.
pub fn result_file_name(set_id: &str, nickname: &str) -> String {
    let slug: String = nickname
        .to_lowercase()
        .chars()
        .map(|c| if c == ' ' { '-' } else { c })
        .collect();
    format!("practice-result-{set_id}-{slug}.json")
}

/// Builds the result file for one completed attempt. `correct[i]` is whether
/// question `i` of the set was answered correctly.
pub fn build_result(set: &PracticeSet, correct: &[bool], nickname_raw: &str) -> Result<SavedFile, String> {
    let nickname = clean_nickname(nickname_raw)?;
    if correct.len() != set.questions.len() {
        return Err(format!(
            "Expected {} answers, got {}.",
            set.questions.len(),
            correct.len()
        ));
    }
    let entries: Vec<Entry> = set
        .questions
        .iter()
        .zip(correct)
        .map(|(q, c)| Entry { question: q.id.clone(), topic: q.topic.clone(), correct: *c })
        .collect();
    let check = check_value(RESULT_FORMAT, &set.id, &nickname, &entries);
    let file = ResultFile {
        format: RESULT_FORMAT.to_string(),
        set_id: set.id.clone(),
        nickname: nickname.clone(),
        entries,
        check,
    };
    let mut contents = serde_json::to_string_pretty(&file)
        .map_err(|e| format!("Could not write the result file: {e}."))?;
    contents.push('\n');
    Ok(SavedFile { file_name: result_file_name(&set.id, &nickname), contents })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::set::tests::sample_set;

    /// The "otter" pattern used across the hand-computed cases:
    /// simplify ✓✓✗, share ✓✗✗, equivalent ✓✓✓, unit-rate ✓✓✗.
    pub(crate) const OTTER: [bool; 12] =
        [true, true, false, true, false, false, true, true, true, true, true, false];

    #[test]
    fn fnv1a64_matches_reference_vectors() {
        // published FNV-1a 64 test vectors
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn canonical_text_layout() {
        let entries = vec![
            Entry { question: "q01".into(), topic: "simplify".into(), correct: true },
            Entry { question: "q02".into(), topic: "simplify".into(), correct: false },
        ];
        assert_eq!(
            canonical_text(RESULT_FORMAT, "s1", "otter", &entries),
            "practice-result/1\nset:s1\nnickname:otter\nq01|simplify|1\nq02|simplify|0\n"
        );
    }

    #[test]
    fn nickname_rules() {
        assert!(clean_nickname("").is_err());
        assert!(clean_nickname("   ").is_err());
        assert_eq!(clean_nickname("otter"), Ok("otter".to_string()));
        assert_eq!(clean_nickname("  otter  "), Ok("otter".to_string()));
        assert_eq!(clean_nickname("Zoë B."), Ok("Zoë B.".to_string()));
        assert!(clean_nickname(&"a".repeat(24)).is_ok());
        let long = clean_nickname(&"a".repeat(25)).unwrap_err();
        assert!(long.contains("at most 24") && long.contains("has 25"), "{long}");
        let slash = clean_nickname("a/b").unwrap_err();
        assert!(slash.contains("'/'"), "{slash}");
        assert!(clean_nickname("a\nb").is_err());
        assert!(clean_nickname("a|b").is_err());
    }

    #[test]
    fn file_name_uses_set_id_and_nickname() {
        assert_eq!(
            result_file_name("ratios-sample-1", "Blue Fox"),
            "practice-result-ratios-sample-1-blue-fox.json"
        );
    }

    #[test]
    fn saved_file_holds_only_allowed_fields() {
        let set = sample_set();
        let saved = build_result(&set, &OTTER, "otter").expect("builds");
        let value: serde_json::Value = serde_json::from_str(&saved.contents).expect("json");
        let top: Vec<&String> = value.as_object().expect("object").keys().collect();
        assert_eq!(top, ["check", "entries", "format", "nickname", "set_id"]);
        let entries = value["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), 12);
        for e in entries {
            let keys: Vec<&String> = e.as_object().expect("entry").keys().collect();
            assert_eq!(keys, ["correct", "question", "topic"]);
        }
        assert_eq!(value["nickname"], "otter");
        assert_eq!(value["set_id"], "ratios-sample-1");
        assert_eq!(value["entries"][2]["question"], "q03");
        assert_eq!(value["entries"][2]["topic"], "simplify");
        assert_eq!(value["entries"][2]["correct"], false);
    }

    #[test]
    fn build_rejects_bad_nickname_and_wrong_length() {
        let set = sample_set();
        assert!(build_result(&set, &OTTER, " ").is_err());
        assert!(build_result(&set, &OTTER[..11], "otter").is_err());
    }
}
