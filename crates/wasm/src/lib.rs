//! practice_wasm: forwards strings between the page and practice_core::api.
//! It decides nothing: each function is a single call into the core, which
//! returns JSON text (`{"ok": ...}` or `{"error": "..."}`).

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn student_start(set_json: &str) -> String {
    practice_core::api::student_start(set_json)
}

#[wasm_bindgen]
pub fn student_answer(set_json: &str, state_json: &str, question: u32, choice: u32) -> String {
    practice_core::api::student_answer(set_json, state_json, question, choice)
}

#[wasm_bindgen]
pub fn student_check(set_json: &str, state_json: &str) -> String {
    practice_core::api::student_check(set_json, state_json)
}

#[wasm_bindgen]
pub fn student_save(set_json: &str, state_json: &str, nickname: &str) -> String {
    practice_core::api::student_save(set_json, state_json, nickname)
}

#[wasm_bindgen]
pub fn teacher_start(set_json: &str) -> String {
    practice_core::api::teacher_start(set_json)
}

#[wasm_bindgen]
pub fn teacher_report(set_json: &str, files_json: &str, threshold: &str) -> String {
    practice_core::api::teacher_report(set_json, files_json, threshold)
}
