//! Boundary tests. They read the core's Cargo.toml and source (and the shim's
//! source) and fail on anything that breaks the architecture rules:
//! (a) the core depends only on serde and serde_json;
//! (e) the core uses no floats, hash maps or clocks (and no randomness);
//! the core performs no input or output;
//! (b) the shim decides nothing.
//!
//! These tests do file I/O themselves; that is fine because they are tests,
//! not the core library. Only `crates/core/src` is held to the core rules.

use std::fs;
use std::path::{Path, PathBuf};

const ALLOWED_DEPENDENCIES: &[&str] = &["serde", "serde_json"];

/// Identifiers that must never appear in the core's code (comments and string
/// literals are removed first), with the rule each one breaks.
const FORBIDDEN_IDENTIFIERS: &[(&str, &str)] = &[
    ("f32", "float"),
    ("f64", "float"),
    ("as_f64", "float"),
    ("from_f64", "float"),
    ("is_f64", "float"),
    ("HashMap", "hash map"),
    ("HashSet", "hash map"),
    ("hash_map", "hash map"),
    ("hash_set", "hash map"),
    ("RandomState", "hash map"),
    ("DefaultHasher", "hash map"),
    ("SystemTime", "clock"),
    ("Instant", "clock"),
    ("UNIX_EPOCH", "clock"),
    ("time", "clock"),
    ("chrono", "clock"),
    ("rand", "randomness"),
    ("thread_rng", "randomness"),
    ("getrandom", "randomness"),
    ("fs", "input/output"),
    ("io", "input/output"),
    ("net", "input/output"),
    ("env", "input/output"),
    ("process", "input/output"),
    ("thread", "input/output"),
    ("File", "input/output"),
    ("stdin", "input/output"),
    ("stdout", "input/output"),
    ("stderr", "input/output"),
    ("println", "input/output"),
    ("print", "input/output"),
    ("eprintln", "input/output"),
    ("eprint", "input/output"),
    ("dbg", "input/output"),
];

/// Words that mean the shim is making a decision instead of forwarding.
const SHIM_DECISION_WORDS: &[&str] = &["if", "else", "match", "for", "while", "loop"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn rust_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let mut entries: Vec<PathBuf> = fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", d.display()))
            .map(|e| e.expect("dir entry").path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Every dependency name declared in a Cargo.toml text, in any section whose
/// name ends in `dependencies` (normal, dev, build, target-specific), in both
/// `[dependencies]` + `name = ...` form and `[dependencies.name]` form.
fn declared_dependencies(toml: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_deps_table = false;
    for raw in toml.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            let header = line.trim_matches(|c| c == '[' || c == ']').trim();
            let parts: Vec<&str> = header.split('.').map(|p| p.trim()).collect();
            in_deps_table = false;
            if let Some(pos) = parts.iter().position(|p| p.ends_with("dependencies")) {
                if pos + 1 < parts.len() {
                    deps.push(parts[pos + 1].trim_matches('"').to_string());
                } else {
                    in_deps_table = true;
                }
            }
            continue;
        }
        if in_deps_table {
            if let Some((key, _)) = line.split_once('=') {
                deps.push(key.trim().trim_matches('"').to_string());
            }
        }
    }
    deps
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Replaces comments and the contents of string and char literals with
/// spaces, keeping newlines so line numbers stay right.
fn strip_comments_and_strings(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        // line comment
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        // block comment (nested)
        if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
            continue;
        }
        // raw string r"..." / r#"..."# (also br"...")
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let prev2 = if i > 1 { Some(chars[i - 2]) } else { None };
        let starts_token = match prev {
            None => true,
            Some(p) if !is_ident_char(p) => true,
            Some('b') => prev2.map(|q| !is_ident_char(q)).unwrap_or(true),
            _ => false,
        };
        if c == 'r' && starts_token && (next == Some('"') || next == Some('#')) {
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                out.push(' ');
                for _ in 0..hashes {
                    out.push(' ');
                }
                out.push(' ');
                j += 1;
                loop {
                    if j >= chars.len() {
                        break;
                    }
                    if chars[j] == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
                        for _ in 0..=hashes {
                            out.push(' ');
                        }
                        j += 1 + hashes;
                        break;
                    }
                    out.push(blank(chars[j]));
                    j += 1;
                }
                i = j;
                continue;
            }
        }
        // normal string
        if c == '"' {
            out.push(' ');
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    out.push(' ');
                    if i + 1 < chars.len() {
                        out.push(blank(chars[i + 1]));
                    }
                    i += 2;
                    continue;
                }
                if chars[i] == '"' {
                    out.push(' ');
                    i += 1;
                    break;
                }
                out.push(blank(chars[i]));
                i += 1;
            }
            continue;
        }
        // char literal (not a lifetime)
        if c == '\'' {
            if next == Some('\\') {
                // skip the backslash and the escaped character, then find the close
                let mut j = i + 3;
                while j < chars.len() && chars[j] != '\'' {
                    j += 1;
                }
                for _ in i..=j.min(chars.len() - 1) {
                    out.push(' ');
                }
                i = j + 1;
                continue;
            }
            if chars.get(i + 2) == Some(&'\'') {
                out.push_str("   ");
                i += 3;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// One problem found by a scan: line number (1-based), the token, and why.
#[derive(Debug, Clone, PartialEq)]
struct Violation {
    line: usize,
    token: String,
    why: String,
}

fn is_float_literal(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    if lower.starts_with("0x") || lower.starts_with("0o") || lower.starts_with("0b") {
        return false;
    }
    if lower.ends_with("f32") || lower.ends_with("f64") {
        return true;
    }
    let b: Vec<char> = lower.chars().collect();
    for k in 1..b.len() {
        let before_is_digit = b[k - 1].is_ascii_digit();
        let after = b.get(k + 1).copied();
        if b[k] == '.' && before_is_digit && after.map(|a| a.is_ascii_digit()).unwrap_or(false) {
            return true;
        }
        if b[k] == 'e'
            && before_is_digit
            && after
                .map(|a| a.is_ascii_digit() || a == '+' || a == '-')
                .unwrap_or(true)
        {
            return true;
        }
    }
    // a trailing dot like `1.` is a float literal too (but `1..` is a range)
    lower.ends_with('.') && !lower.ends_with("..")
}

/// Scans stripped Rust code for forbidden identifiers and float literals.
fn scan_core_code(src: &str) -> Vec<Violation> {
    let code = strip_comments_and_strings(src);
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            if c.is_ascii_digit() && !prev.map(|p| is_ident_char(p) || p == '.').unwrap_or(false) {
                let start = i;
                while i < chars.len() && (is_ident_char(chars[i]) || chars[i] == '.') {
                    i += 1;
                }
                let token: String = chars[start..i].iter().collect();
                if is_float_literal(&token) {
                    found.push(Violation { line: n + 1, token, why: "float".into() });
                }
                continue;
            }
            if is_ident_char(c) && !c.is_ascii_digit() {
                let start = i;
                while i < chars.len() && is_ident_char(chars[i]) {
                    i += 1;
                }
                let token: String = chars[start..i].iter().collect();
                if let Some((_, why)) = FORBIDDEN_IDENTIFIERS.iter().find(|(w, _)| *w == token) {
                    found.push(Violation { line: n + 1, token, why: (*why).to_string() });
                }
                continue;
            }
            i += 1;
        }
    }
    found
}

/// Scans stripped shim code for decision keywords and the `?` operator.
fn scan_shim_code(src: &str) -> Vec<Violation> {
    let code = strip_comments_and_strings(src);
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        for word in line.split(|c: char| !is_ident_char(c)) {
            if SHIM_DECISION_WORDS.contains(&word) {
                found.push(Violation { line: n + 1, token: word.to_string(), why: "decision".into() });
            }
        }
        if line.contains('?') {
            found.push(Violation { line: n + 1, token: "?".into(), why: "decision".into() });
        }
    }
    found
}

fn report(path: &Path, vs: &[Violation]) -> Vec<String> {
    vs.iter()
        .map(|v| format!("{}:{}: `{}` ({})", path.display(), v.line, v.token, v.why))
        .collect()
}

#[test]
fn core_dependencies_are_only_serde_and_serde_json() {
    let path = repo_root().join("crates/core/Cargo.toml");
    let toml = fs::read_to_string(&path).expect("read core Cargo.toml");
    let deps = declared_dependencies(&toml);
    assert!(deps.contains(&"serde".to_string()), "expected serde in {deps:?}");
    let outside: Vec<&String> = deps
        .iter()
        .filter(|d| !ALLOWED_DEPENDENCIES.contains(&d.as_str()))
        .collect();
    assert!(
        outside.is_empty(),
        "{}: dependencies outside the list {:?}: {:?}",
        path.display(),
        ALLOWED_DEPENDENCIES,
        outside
    );
    assert!(
        !repo_root().join("crates/core/build.rs").exists(),
        "the core must not have a build script"
    );
}

#[test]
fn core_source_has_no_float_hashmap_clock_or_io() {
    let src_dir = repo_root().join("crates/core/src");
    let files = rust_files_under(&src_dir);
    assert!(!files.is_empty(), "no source files found under {}", src_dir.display());
    let mut problems = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).expect("read source");
        problems.extend(report(f, &scan_core_code(&text)));
    }
    assert!(problems.is_empty(), "core rule violations:\n{}", problems.join("\n"));
}

#[test]
fn shim_decides_nothing() {
    let src_dir = repo_root().join("crates/wasm/src");
    let files = rust_files_under(&src_dir);
    assert!(!files.is_empty(), "no shim source found");
    let mut problems = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).expect("read shim source");
        problems.extend(report(f, &scan_shim_code(&text)));
    }
    assert!(problems.is_empty(), "the shim makes decisions:\n{}", problems.join("\n"));
}

#[test]
fn dependency_reader_sees_every_form() {
    let toml = "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\n\n[dev-dependencies]\nregex = \"1\"\n\n[build-dependencies.cc]\nversion = \"1\"\n\n[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n";
    assert_eq!(declared_dependencies(toml), vec!["serde", "regex", "cc", "libc"]);
}

#[test]
fn scanner_catches_each_kind() {
    let cases = [
        ("let x = 0.5;", "float"),
        ("let y: f64 = 2;", "float"),
        ("let z = 1e3;", "float"),
        ("let w = 3f32;", "float"),
        ("let m = HashMap::new();", "hash map"),
        ("use std::collections::hash_map::Entry;", "hash map"),
        ("let t = std::time::SystemTime::now();", "clock"),
        ("let i = Instant::now();", "clock"),
        ("let s = std::fs::read_to_string(p);", "input/output"),
        ("println!(\"hi\");", "input/output"),
        ("use std::{fs, io};", "input/output"),
    ];
    for (code, why) in cases {
        let found = scan_core_code(code);
        assert!(
            found.iter().any(|v| v.why == why),
            "expected a `{why}` violation in {code:?}, got {found:?}"
        );
    }
}

#[test]
fn scanner_ignores_comments_strings_hex_and_ranges() {
    let code = r##"
// f64 HashMap 0.5 in a line comment
/* SystemTime 1.5 /* nested f32 */ still comment */
let a = "0.5 f64 HashMap";
let b = r#"{"rate": 0.25, "HashMap": "x"}"#;
let c = 0xcbf29ce484222325u64;
for i in 0..10 { let _ = pair.0; }
let d = 'x';
let e = '\'';
fn f<'a>(s: &'a str) -> &'a str { s }
let g = 5.max(3);
let h = 100u32 * 7;
"##;
    let found = scan_core_code(code);
    assert!(found.is_empty(), "false positives: {found:?}");
}

#[test]
fn shim_scanner_catches_decisions() {
    assert!(!scan_shim_code("pub fn f(x: &str) -> String { if x.is_empty() { a() } else { b() } }").is_empty());
    assert!(!scan_shim_code("pub fn f(x: &str) -> Result<String, E> { Ok(g(x)?) }").is_empty());
    assert!(scan_shim_code("// if this were a decision\npub fn f(x: &str) -> String { core::api::f(x) }").is_empty());
}
