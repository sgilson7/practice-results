//! The teacher view: many result files in, one report out.
//!
//! Per topic (in set order): correct answers, attempts, floored percentage,
//! and whether to suggest reteaching it (below the threshold). Per topic, the
//! questions sorted by how often they were missed (ties in set order), with
//! the highest non-zero count marked "most missed" (all of them on a tie).
//! Rejected files are listed with their reason and never counted.

use serde::{Deserialize, Serialize};

use crate::rate::{below_threshold, parse_threshold, percent_floor};
use crate::result::{read_result, AcceptedResult};
use crate::set::PracticeSet;

/// One file the teacher opened: its name and its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputFile {
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AcceptedFile {
    pub name: String,
    pub nickname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RejectedFile {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QuestionStat {
    pub id: String,
    pub number: u32,
    pub prompt: String,
    pub answer: String,
    pub missed: u32,
    pub attempts: u32,
    pub most_missed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TopicStat {
    pub id: String,
    pub name: String,
    pub correct: u32,
    pub attempts: u32,
    /// floor(100 * correct / attempts); None with no attempts.
    pub percent: Option<u32>,
    /// Suggest reteaching; None with no attempts or no valid threshold.
    pub reteach: Option<bool>,
    /// The topic's questions, most missed first.
    pub questions: Vec<QuestionStat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub set_id: String,
    pub title: String,
    pub sample: bool,
    pub threshold: Option<u32>,
    pub threshold_error: Option<String>,
    pub counted: u32,
    pub not_counted: u32,
    pub accepted: Vec<AcceptedFile>,
    pub rejected: Vec<RejectedFile>,
    pub notices: Vec<String>,
    pub topics: Vec<TopicStat>,
    /// Names of the topics to reteach, in set order.
    pub reteach_topics: Vec<String>,
}

/// Builds the report for the files the teacher opened.
pub fn build_report(set: &PracticeSet, files: &[InputFile], threshold_text: &str) -> Report {
    let (threshold, threshold_error) = match parse_threshold(threshold_text) {
        Ok(t) => (Some(t), None),
        Err(e) => (None, Some(e)),
    };

    let mut accepted: Vec<(AcceptedFile, AcceptedResult)> = Vec::new();
    let mut rejected: Vec<RejectedFile> = Vec::new();
    let mut notices: Vec<String> = Vec::new();
    for f in files {
        match read_result(set, &f.text) {
            Err(reason) => rejected.push(RejectedFile { name: f.name.clone(), reason }),
            Ok(r) => {
                let twin = accepted
                    .iter()
                    .find(|(_, a)| a.nickname == r.nickname && a.correct == r.correct);
                if let Some((first, _)) = twin {
                    notices.push(format!(
                        "{} has the same nickname and answers as {}, so it was counted once.",
                        f.name, first.name
                    ));
                    continue;
                }
                accepted.push((AcceptedFile { name: f.name.clone(), nickname: r.nickname.clone() }, r));
            }
        }
    }

    let mut seen: Vec<&str> = Vec::new();
    for (i, (_, r)) in accepted.iter().enumerate() {
        if seen.contains(&r.nickname.as_str()) {
            continue;
        }
        seen.push(&r.nickname);
        let names: Vec<&str> = accepted[i..]
            .iter()
            .filter(|(_, o)| o.nickname == r.nickname)
            .map(|(f, _)| f.name.as_str())
            .collect();
        if names.len() > 1 {
            notices.push(format!(
                "The nickname \"{}\" is in {} files with different answers ({}). They may be repeated attempts by one student or different students; each file was counted.",
                r.nickname,
                names.len(),
                names.join(", ")
            ));
        }
    }

    let counted = accepted.len() as u32;
    let n = set.questions.len();
    let mut right = vec![0u32; n];
    for (_, r) in &accepted {
        for (i, c) in r.correct.iter().enumerate() {
            if *c {
                right[i] += 1;
            }
        }
    }

    let topics: Vec<TopicStat> = set
        .topics
        .iter()
        .map(|t| {
            let members: Vec<usize> = (0..n).filter(|i| set.questions[*i].topic == t.id).collect();
            let correct: u32 = members.iter().map(|i| right[*i]).sum();
            let attempts = counted * members.len() as u32;
            let mut questions: Vec<QuestionStat> = members
                .iter()
                .map(|i| {
                    let q = &set.questions[*i];
                    QuestionStat {
                        id: q.id.clone(),
                        number: *i as u32 + 1,
                        prompt: q.prompt.clone(),
                        answer: q.answer.clone(),
                        missed: counted - right[*i],
                        attempts: counted,
                        most_missed: false,
                    }
                })
                .collect();
            // stable sort: ties keep set order
            questions.sort_by(|a, b| b.missed.cmp(&a.missed));
            let top = questions.first().map(|q| q.missed).unwrap_or(0);
            if top > 0 {
                for q in &mut questions {
                    q.most_missed = q.missed == top;
                }
            }
            TopicStat {
                id: t.id.clone(),
                name: t.name.clone(),
                correct,
                attempts,
                percent: percent_floor(correct, attempts),
                reteach: threshold.and_then(|th| below_threshold(correct, attempts, th)),
                questions,
            }
        })
        .collect();

    let reteach_topics = topics
        .iter()
        .filter(|t| t.reteach == Some(true))
        .map(|t| t.name.clone())
        .collect();

    Report {
        set_id: set.id.clone(),
        title: set.title.clone(),
        sample: set.sample,
        threshold,
        threshold_error,
        counted,
        not_counted: rejected.len() as u32,
        accepted: accepted.into_iter().map(|(f, _)| f).collect(),
        rejected,
        notices,
        topics,
        reteach_topics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::build_result;
    use crate::result::tests::{ALTERED_FILE, KESTREL_FILE, MAPLE_FILE, OTTER_FILE};
    use crate::set::tests::sample_set;

    fn file(name: &str, text: &str) -> InputFile {
        InputFile { name: name.to_string(), text: text.to_string() }
    }

    fn three() -> Vec<InputFile> {
        vec![
            file("otter.json", OTTER_FILE),
            file("kestrel.json", KESTREL_FILE),
            file("maple.json", MAPLE_FILE),
        ]
    }

    fn rates(r: &Report) -> Vec<(&str, u32, u32, Option<u32>)> {
        r.topics.iter().map(|t| (t.id.as_str(), t.correct, t.attempts, t.percent)).collect()
    }

    fn reteach(r: &Report) -> Vec<Option<bool>> {
        r.topics.iter().map(|t| t.reteach).collect()
    }

    #[test]
    fn three_files_hand_computed() {
        // by hand (tests/fixtures/make_fixtures.py): simplify 2+3+1 = 6 of 9,
        // share 1+1+1 = 3 of 9, equivalent 3+2+3 = 8 of 9, unit-rate 2+2+3 = 7 of 9
        let r = build_report(&sample_set(), &three(), "70");
        assert_eq!(
            rates(&r),
            vec![
                ("simplify", 6, 9, Some(66)),
                ("share", 3, 9, Some(33)),
                ("equivalent", 8, 9, Some(88)),
                ("unit-rate", 7, 9, Some(77)),
            ]
        );
        assert_eq!((r.counted, r.not_counted), (3, 0));
        assert!(r.notices.is_empty());
    }

    #[test]
    fn reteach_at_70_default() {
        let r = build_report(&sample_set(), &three(), "70");
        assert_eq!(r.threshold, Some(70));
        assert_eq!(reteach(&r), vec![Some(true), Some(true), Some(false), Some(false)]);
        assert_eq!(r.reteach_topics, vec!["Simplifying ratios", "Sharing in a ratio"]);
    }

    #[test]
    fn reteach_at_other_thresholds() {
        let set = sample_set();
        let at80 = build_report(&set, &three(), "80");
        assert_eq!(reteach(&at80), vec![Some(true), Some(true), Some(false), Some(true)]);
        let at0 = build_report(&set, &three(), "0");
        assert_eq!(reteach(&at0), vec![Some(false); 4]);
        let at100 = build_report(&set, &three(), "100");
        assert_eq!(reteach(&at100), vec![Some(true); 4]);
        let at66 = build_report(&set, &three(), "66"); // 6/9 = 66.6..% is not below 66
        assert_eq!(reteach(&at66), vec![Some(false), Some(true), Some(false), Some(false)]);
    }

    #[test]
    fn most_missed_per_topic() {
        // misses by hand: q01 0, q02 1, q03 2 | q04 1, q05 3, q06 2 |
        // q07 0, q08 0, q09 1 | q10 0, q11 0, q12 2
        let r = build_report(&sample_set(), &three(), "70");
        let order: Vec<Vec<(&str, u32, bool)>> = r
            .topics
            .iter()
            .map(|t| t.questions.iter().map(|q| (q.id.as_str(), q.missed, q.most_missed)).collect())
            .collect();
        assert_eq!(
            order,
            vec![
                vec![("q03", 2, true), ("q02", 1, false), ("q01", 0, false)],
                vec![("q05", 3, true), ("q06", 2, false), ("q04", 1, false)],
                vec![("q09", 1, true), ("q07", 0, false), ("q08", 0, false)],
                vec![("q12", 2, true), ("q10", 0, false), ("q11", 0, false)],
            ]
        );
        assert_eq!(r.topics[1].questions[0].answer, "$20");
        assert_eq!(r.topics[1].questions[0].attempts, 3);
    }

    #[test]
    fn most_missed_ties_are_all_marked() {
        // otter + maple, share: q04 0 misses, q05 2, q06 2
        let files = vec![file("otter.json", OTTER_FILE), file("maple.json", MAPLE_FILE)];
        let r = build_report(&sample_set(), &files, "70");
        let share: Vec<(&str, u32, bool)> =
            r.topics[1].questions.iter().map(|q| (q.id.as_str(), q.missed, q.most_missed)).collect();
        assert_eq!(share, vec![("q05", 2, true), ("q06", 2, true), ("q04", 0, false)]);
    }

    #[test]
    fn rejected_files_are_listed_and_not_counted() {
        let mut files = three();
        files.push(file("altered-otter.json", ALTERED_FILE));
        files.push(file("notes.txt", "this is not a result file"));
        let r = build_report(&sample_set(), &files, "70");
        assert_eq!(rates(&r), rates(&build_report(&sample_set(), &three(), "70")));
        assert_eq!((r.counted, r.not_counted), (3, 2));
        assert_eq!(r.rejected[0].name, "altered-otter.json");
        assert!(r.rejected[0].reason.contains("check value does not match"), "{}", r.rejected[0].reason);
        assert_eq!(r.rejected[1].name, "notes.txt");
        assert!(r.rejected[1].reason.contains("not a result file"), "{}", r.rejected[1].reason);
    }

    #[test]
    fn no_files_no_data() {
        let r = build_report(&sample_set(), &[], "70");
        assert_eq!(r.counted, 0);
        assert_eq!(r.topics.len(), 4, "every topic is still listed, with no data");
        for t in &r.topics {
            assert_eq!((t.correct, t.attempts, t.percent, t.reteach), (0, 0, None, None));
            assert!(t.questions.iter().all(|q| !q.most_missed));
        }
        assert!(r.reteach_topics.is_empty());
    }

    #[test]
    fn duplicate_file_counted_once() {
        let mut files = three();
        files.push(file("otter (1).json", OTTER_FILE));
        let r = build_report(&sample_set(), &files, "70");
        assert_eq!(rates(&r), rates(&build_report(&sample_set(), &three(), "70")));
        assert_eq!(r.counted, 3);
        assert_eq!(
            r.notices,
            vec!["otter (1).json has the same nickname and answers as otter.json, so it was counted once."]
        );
    }

    #[test]
    fn same_nickname_different_answers_both_counted_with_notice() {
        let set = sample_set();
        let kestrel_answers = [true, true, true, false, false, true, true, true, false, true, true, false];
        let second = build_result(&set, &kestrel_answers, "otter").unwrap();
        let files = vec![file("otter.json", OTTER_FILE), file("otter-2.json", &second.contents)];
        let r = build_report(&set, &files, "70");
        assert_eq!(r.counted, 2);
        assert_eq!(r.notices.len(), 1);
        assert!(
            r.notices[0].starts_with("The nickname \"otter\" is in 2 files with different answers (otter.json, otter-2.json)."),
            "{}",
            r.notices[0]
        );
    }

    #[test]
    fn invalid_threshold_gives_message_and_no_suggestions() {
        let r = build_report(&sample_set(), &three(), "seventy");
        assert_eq!(r.threshold, None);
        assert!(r.threshold_error.as_deref().unwrap_or("").contains("whole number from 0 to 100"));
        assert_eq!(reteach(&r), vec![None; 4]);
        assert_eq!(r.topics[0].percent, Some(66));
        assert!(r.reteach_topics.is_empty());
    }
}
