//! The JSON boundary used by the wasm shim. Every function takes strings (and
//! plain integers) and returns a JSON string: `{"ok": ...}` or
//! `{"error": "..."}`. Nothing here panics on bad input; errors are messages.

use serde::Serialize;

use crate::rate::DEFAULT_THRESHOLD;
use crate::report::{build_report, InputFile};
use crate::set::parse_set;
use crate::student::{self, StudentState, StudentView};

#[derive(Serialize)]
struct Ok<'a, T: Serialize> {
    ok: &'a T,
}

#[derive(Serialize)]
struct Error<'a> {
    error: &'a str,
}

fn error(message: &str) -> String {
    serde_json::to_string(&Error { error: message })
        .unwrap_or_else(|_| "{\"error\":\"Unknown error.\"}".to_string())
}

fn respond<T: Serialize>(result: Result<T, String>) -> String {
    match result {
        Ok(value) => serde_json::to_string(&Ok { ok: &value })
            .unwrap_or_else(|e| error(&format!("Could not encode the answer: {e}."))),
        Err(message) => error(&message),
    }
}

#[derive(Serialize)]
struct StudentStep {
    state: StudentState,
    view: StudentView,
}

#[derive(Serialize)]
struct TeacherStart {
    set_id: String,
    title: String,
    sample: bool,
    note: String,
    threshold: String,
}

fn parse_state(text: &str) -> Result<StudentState, String> {
    serde_json::from_str(text)
        .map_err(|e| format!("This attempt could not be read ({e}). Start again."))
}

fn step(set: &crate::set::PracticeSet, state: StudentState) -> Result<StudentStep, String> {
    let view = student::view(set, &state)?;
    Ok(StudentStep { state, view })
}

/// A fresh attempt: `{"ok": {"state": ..., "view": ...}}`.
pub fn student_start(set_json: &str) -> String {
    respond(parse_set(set_json).and_then(|set| {
        let state = student::start(&set);
        step(&set, state)
    }))
}

/// Records one choice: `{"ok": {"state": ..., "view": ...}}`.
pub fn student_answer(set_json: &str, state_json: &str, question: u32, choice: u32) -> String {
    respond(parse_set(set_json).and_then(|set| {
        let state = parse_state(state_json)?;
        let next = student::answer(&set, &state, question, choice)?;
        step(&set, next)
    }))
}

/// Checks and locks the answers: `{"ok": {"state": ..., "view": ...}}`.
pub fn student_check(set_json: &str, state_json: &str) -> String {
    respond(parse_set(set_json).and_then(|set| {
        let state = parse_state(state_json)?;
        let next = student::check(&set, &state)?;
        step(&set, next)
    }))
}

/// The result file for a checked attempt: `{"ok": {"file_name", "contents"}}`.
pub fn student_save(set_json: &str, state_json: &str, nickname: &str) -> String {
    respond(parse_set(set_json).and_then(|set| {
        let state = parse_state(state_json)?;
        student::save(&set, &state, nickname)
    }))
}

/// What the teacher view shows before any file is opened, including the
/// default threshold as text for the input box.
pub fn teacher_start(set_json: &str) -> String {
    respond(parse_set(set_json).map(|set| TeacherStart {
        set_id: set.id,
        title: set.title,
        sample: set.sample,
        note: set.note,
        threshold: DEFAULT_THRESHOLD.to_string(),
    }))
}

/// The report for the opened files. `files_json` is `[{"name", "text"}, ...]`.
pub fn teacher_report(set_json: &str, files_json: &str, threshold: &str) -> String {
    respond(parse_set(set_json).and_then(|set| {
        let files: Vec<InputFile> = serde_json::from_str(files_json)
            .map_err(|e| format!("The opened files could not be passed to the report ({e})."))?;
        Ok(build_report(&set, &files, threshold))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::tests::{KESTREL_FILE, MAPLE_FILE, OTTER, OTTER_FILE};
    use crate::set::tests::SAMPLE;
    use serde_json::Value;

    fn ok(text: &str) -> Value {
        let v: Value = serde_json::from_str(text).unwrap_or_else(|e| panic!("not JSON ({e}): {text:?}"));
        assert!(v.get("error").is_none(), "unexpected error: {text}");
        v["ok"].clone()
    }

    fn err(text: &str) -> String {
        let v: Value = serde_json::from_str(text).unwrap_or_else(|e| panic!("not JSON ({e}): {text:?}"));
        v["error"].as_str().unwrap_or_else(|| panic!("expected an error: {text}")).to_string()
    }

    #[test]
    fn student_round_trip_produces_the_fixture_file() {
        let mut out = ok(&student_start(SAMPLE));
        let set = parse_set(SAMPLE).unwrap();
        for (i, right) in OTTER.iter().enumerate() {
            let key = set.answer_index(i).unwrap();
            let pick = if *right { key } else if key == 0 { 1 } else { 0 };
            out = ok(&student_answer(SAMPLE, &out["state"].to_string(), i as u32, pick as u32));
        }
        assert_eq!(out["view"]["answered"], 12);
        let checked = ok(&student_check(SAMPLE, &out["state"].to_string()));
        assert_eq!(checked["view"]["scores"][1]["percent"], 33);
        let saved = ok(&student_save(SAMPLE, &checked["state"].to_string(), "otter"));
        assert_eq!(saved["file_name"], "practice-result-ratios-sample-1-otter.json");
        assert_eq!(saved["contents"], OTTER_FILE);
    }

    #[test]
    fn check_with_missing_answers_is_an_error_message() {
        let start = ok(&student_start(SAMPLE));
        let msg = err(&student_check(SAMPLE, &start["state"].to_string()));
        assert!(msg.starts_with("Answer every question before checking."), "{msg}");
    }

    #[test]
    fn garbage_input_gives_error_messages_not_panics() {
        for bad in ["", "{", "null", "[]", "{\"answers\": 5}"] {
            err(&student_start(bad));
            err(&student_answer(SAMPLE, bad, 0, 0));
            err(&student_check(SAMPLE, bad));
            err(&student_save(SAMPLE, bad, "otter"));
            err(&teacher_start(bad));
            err(&teacher_report(bad, "[]", "70"));
        }
        // "[]" is a valid (empty) list of opened files, so it is not garbage here
        for bad in ["", "{", "null", "{\"answers\": 5}", "[{\"name\": 1}]", "[{\"name\": \"a\"}]"] {
            err(&teacher_report(SAMPLE, bad, "70"));
        }
        let start = ok(&student_start(SAMPLE));
        err(&student_answer(SAMPLE, &start["state"].to_string(), 99, 0));
        err(&student_answer(SAMPLE, &start["state"].to_string(), u32::MAX, u32::MAX));
    }

    #[test]
    fn teacher_start_gives_default_threshold_text() {
        let start = ok(&teacher_start(SAMPLE));
        assert_eq!(start["threshold"], "70");
        assert_eq!(start["sample"], true);
    }

    #[test]
    fn teacher_report_through_json() {
        let files = serde_json::to_string(&vec![
            InputFile { name: "otter.json".into(), text: OTTER_FILE.into() },
            InputFile { name: "kestrel.json".into(), text: KESTREL_FILE.into() },
            InputFile { name: "maple.json".into(), text: MAPLE_FILE.into() },
        ])
        .unwrap();
        let report = ok(&teacher_report(SAMPLE, &files, "70"));
        let percents: Vec<i64> = report["topics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["percent"].as_i64().unwrap())
            .collect();
        assert_eq!(percents, vec![66, 33, 88, 77]);
        assert_eq!(report["reteach_topics"], serde_json::json!(["Simplifying ratios", "Sharing in a ratio"]));
        let empty = ok(&teacher_report(SAMPLE, "[]", "70"));
        assert!(empty["topics"][0]["percent"].is_null());
        assert!(empty["topics"][0]["reteach"].is_null());
    }
}
