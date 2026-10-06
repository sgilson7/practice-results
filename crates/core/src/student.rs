//! The student view: one attempt at a practice set.
//!
//! The page holds a `StudentState` that only this module creates and changes.
//! Checking needs every question answered and then locks the answers, so the
//! saved result is the attempt that was checked. "Start again" is a new state.
//! After checking, the student sees a score per topic (not per question).

use serde::{Deserialize, Serialize};

use crate::rate::percent_floor;
use crate::result::{build_result, SavedFile};
use crate::set::PracticeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudentState {
    /// Chosen choice index per question, in set order; `None` = not answered.
    pub answers: Vec<Option<u32>>,
    /// Set once the answers are checked; the answers are then locked.
    pub checked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChoiceView {
    pub index: u32,
    pub text: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuestionView {
    pub index: u32,
    pub number: u32,
    pub id: String,
    pub topic_name: String,
    pub prompt: String,
    pub choices: Vec<ChoiceView>,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TopicScore {
    pub topic: String,
    pub name: String,
    pub correct: u32,
    pub attempts: u32,
    pub percent: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StudentView {
    pub set_id: String,
    pub title: String,
    pub sample: bool,
    pub note: String,
    pub questions: Vec<QuestionView>,
    pub answered: u32,
    pub total: u32,
    pub checked: bool,
    pub can_check: bool,
    pub can_save: bool,
    /// Per-topic scores; empty until the answers are checked.
    pub scores: Vec<TopicScore>,
    /// Overall score; `None` until the answers are checked.
    pub overall: Option<TopicScore>,
}

/// A fresh attempt: nothing answered, nothing locked.
pub fn start(set: &PracticeSet) -> StudentState {
    StudentState { answers: vec![None; set.questions.len()], checked: false }
}

fn validate_state(set: &PracticeSet, state: &StudentState) -> Result<(), String> {
    if state.answers.len() != set.questions.len() {
        return Err(format!(
            "This attempt has {} answers but the set has {} questions. Start again.",
            state.answers.len(),
            set.questions.len()
        ));
    }
    for (q, a) in set.questions.iter().zip(&state.answers) {
        if let Some(choice) = a {
            if *choice as usize >= q.choices.len() {
                return Err(format!("Question \"{}\" has no choice {}. Start again.", q.id, choice));
            }
        }
    }
    Ok(())
}

/// Records the student's choice for one question.
pub fn answer(set: &PracticeSet, state: &StudentState, question: u32, choice: u32) -> Result<StudentState, String> {
    validate_state(set, state)?;
    if state.checked {
        return Err(
            "Your answers are locked because you have checked them. Choose \"Start again\" for a new attempt."
                .to_string(),
        );
    }
    let q = set
        .questions
        .get(question as usize)
        .ok_or_else(|| format!("There is no question {}.", u64::from(question) + 1))?;
    if choice as usize >= q.choices.len() {
        return Err(format!("Question {} has no choice {}.", u64::from(question) + 1, choice));
    }
    let mut next = state.clone();
    next.answers[question as usize] = Some(choice);
    Ok(next)
}

/// Checks the answers: every question must be answered; then they lock.
pub fn check(set: &PracticeSet, state: &StudentState) -> Result<StudentState, String> {
    validate_state(set, state)?;
    let missing: Vec<String> = state
        .answers
        .iter()
        .enumerate()
        .filter(|(_, a)| a.is_none())
        .map(|(i, _)| (i + 1).to_string())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "Answer every question before checking. Not answered yet: {}.",
            missing.join(", ")
        ));
    }
    let mut next = state.clone();
    next.checked = true;
    Ok(next)
}

/// Whether each question was answered correctly (unanswered = not correct).
pub fn correct_flags(set: &PracticeSet, state: &StudentState) -> Vec<bool> {
    (0..set.questions.len())
        .map(|i| {
            let chosen = state.answers.get(i).copied().flatten().map(|c| c as usize);
            chosen.is_some() && chosen == set.answer_index(i)
        })
        .collect()
}

/// Score per topic, in the set's topic order.
pub fn topic_scores(set: &PracticeSet, correct: &[bool]) -> Vec<TopicScore> {
    set.topics
        .iter()
        .map(|t| {
            let mut right = 0u32;
            let mut attempts = 0u32;
            for (q, c) in set.questions.iter().zip(correct) {
                if q.topic == t.id {
                    attempts += 1;
                    if *c {
                        right += 1;
                    }
                }
            }
            TopicScore {
                topic: t.id.clone(),
                name: t.name.clone(),
                correct: right,
                attempts,
                percent: percent_floor(right, attempts),
            }
        })
        .collect()
}

/// Everything the page needs to draw the student view.
pub fn view(set: &PracticeSet, state: &StudentState) -> Result<StudentView, String> {
    validate_state(set, state)?;
    let questions = set
        .questions
        .iter()
        .enumerate()
        .map(|(i, q)| QuestionView {
            index: i as u32,
            number: i as u32 + 1,
            id: q.id.clone(),
            topic_name: set.topic_name(&q.topic).to_string(),
            prompt: q.prompt.clone(),
            choices: q
                .choices
                .iter()
                .enumerate()
                .map(|(j, text)| ChoiceView {
                    index: j as u32,
                    text: text.clone(),
                    selected: state.answers[i] == Some(j as u32),
                })
                .collect(),
            locked: state.checked,
        })
        .collect();
    let answered = state.answers.iter().filter(|a| a.is_some()).count() as u32;
    let (scores, overall) = if state.checked {
        let flags = correct_flags(set, state);
        let right = flags.iter().filter(|c| **c).count() as u32;
        let total = flags.len() as u32;
        let overall = TopicScore {
            topic: String::new(),
            name: "All topics".to_string(),
            correct: right,
            attempts: total,
            percent: percent_floor(right, total),
        };
        (topic_scores(set, &flags), Some(overall))
    } else {
        (Vec::new(), None)
    };
    Ok(StudentView {
        set_id: set.id.clone(),
        title: set.title.clone(),
        sample: set.sample,
        note: set.note.clone(),
        questions,
        answered,
        total: set.questions.len() as u32,
        checked: state.checked,
        can_check: !state.checked,
        can_save: state.checked,
        scores,
        overall,
    })
}

/// The result file for a checked attempt.
pub fn save(set: &PracticeSet, state: &StudentState, nickname: &str) -> Result<SavedFile, String> {
    validate_state(set, state)?;
    if !state.checked {
        return Err("Check your answers before saving.".to_string());
    }
    build_result(set, &correct_flags(set, state), nickname)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::result::tests::OTTER;
    use crate::set::tests::sample_set;

    /// Answers every question: the key when `pattern[i]` is true, otherwise
    /// the first choice that is not the key.
    pub(crate) fn answer_pattern(set: &PracticeSet, pattern: &[bool]) -> StudentState {
        let mut state = start(set);
        for (i, right) in pattern.iter().enumerate() {
            let key = set.answer_index(i).expect("key");
            let pick = if *right { key } else if key == 0 { 1 } else { 0 };
            state = answer(set, &state, i as u32, pick as u32).expect("answer");
        }
        state
    }

    #[test]
    fn new_attempt_has_no_answers_and_is_unlocked() {
        let set = sample_set();
        let state = start(&set);
        assert_eq!(state.answers, vec![None; 12]);
        assert!(!state.checked);
        let v = view(&set, &state).unwrap();
        assert_eq!((v.answered, v.total), (0, 12));
        assert!(v.scores.is_empty() && v.overall.is_none());
        assert!(v.questions.iter().all(|q| !q.locked));
        assert!(v.can_check && !v.can_save);
    }

    #[test]
    fn answer_records_choice_and_rejects_out_of_range() {
        let set = sample_set();
        let state = answer(&set, &start(&set), 0, 1).unwrap();
        assert_eq!(state.answers[0], Some(1));
        let v = view(&set, &state).unwrap();
        assert!(v.questions[0].choices[1].selected && !v.questions[0].choices[0].selected);
        assert_eq!(answer(&set, &state, 12, 0).unwrap_err(), "There is no question 13.");
        assert_eq!(answer(&set, &state, 0, 4).unwrap_err(), "Question 1 has no choice 4.");
    }

    #[test]
    fn check_requires_every_answer() {
        let set = sample_set();
        let mut state = answer_pattern(&set, &OTTER);
        for i in [3, 6, 10] {
            state.answers[i] = None;
        }
        assert_eq!(
            check(&set, &state).unwrap_err(),
            "Answer every question before checking. Not answered yet: 4, 7, 11."
        );
    }

    #[test]
    fn answers_are_locked_after_check() {
        let set = sample_set();
        let state = check(&set, &answer_pattern(&set, &OTTER)).unwrap();
        assert!(state.checked);
        let err = answer(&set, &state, 0, 0).unwrap_err();
        assert!(err.contains("locked"), "{err}");
        let v = view(&set, &state).unwrap();
        assert!(v.questions.iter().all(|q| q.locked));
        assert!(!v.can_check && v.can_save);
    }

    #[test]
    fn topic_scores_hand_computed() {
        // otter: simplify 2 of 3, share 1 of 3, equivalent 3 of 3, unit-rate 2 of 3
        let set = sample_set();
        let state = check(&set, &answer_pattern(&set, &OTTER)).unwrap();
        let v = view(&set, &state).unwrap();
        let got: Vec<(&str, u32, u32, Option<u32>)> = v
            .scores
            .iter()
            .map(|s| (s.topic.as_str(), s.correct, s.attempts, s.percent))
            .collect();
        assert_eq!(
            got,
            vec![
                ("simplify", 2, 3, Some(66)),
                ("share", 1, 3, Some(33)),
                ("equivalent", 3, 3, Some(100)),
                ("unit-rate", 2, 3, Some(66)),
            ]
        );
        let overall = v.overall.unwrap();
        assert_eq!((overall.correct, overall.attempts, overall.percent), (8, 12, Some(66)));
    }

    #[test]
    fn save_requires_check() {
        let set = sample_set();
        let state = answer_pattern(&set, &OTTER);
        assert_eq!(save(&set, &state, "otter").unwrap_err(), "Check your answers before saving.");
        let saved = save(&set, &check(&set, &state).unwrap(), "otter").unwrap();
        assert_eq!(saved.file_name, "practice-result-ratios-sample-1-otter.json");
    }

    #[test]
    fn tampered_state_is_rejected() {
        let set = sample_set();
        let mut state = start(&set);
        state.answers.pop();
        assert!(view(&set, &state).unwrap_err().contains("11 answers"));
        let mut state = start(&set);
        state.answers[0] = Some(9);
        assert!(check(&set, &state).is_err());
    }
}
