// Draws what the core sends. This file holds no rule: marking, scores,
// thresholds, nickname rules and file checks are all made by practice_core
// (through the wasm shim). Every core call returns JSON text:
// {"ok": ...} or {"error": "..."}.
import init, * as core from "./pkg/practice_wasm.js";

const $ = (id) => document.getElementById(id);

let setJson = "";
let studentState = "";
let builtQuestions = false;
let lastDownloadUrl = "";
let reportSeq = 0;

function el(tag, text, className) {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
}

function showView(name) {
  const student = name === "student";
  $("student-view").hidden = !student;
  $("teacher-view").hidden = student;
  $("tab-student").setAttribute("aria-pressed", String(student));
  $("tab-teacher").setAttribute("aria-pressed", String(!student));
}

// ---------- student view ----------

function buildQuestions(view) {
  const list = $("question-list");
  list.replaceChildren();
  for (const q of view.questions) {
    const item = el("li");
    const box = el("fieldset");
    box.id = "question-" + q.id;
    const legend = el("legend");
    legend.append(el("span", "Question " + q.number + ". ", "number"), el("span", q.prompt));
    legend.append(el("span", q.topic_name, "topic"));
    box.append(legend);
    for (const c of q.choices) {
      const label = el("label");
      const input = el("input");
      input.type = "radio";
      input.name = "q-" + q.id;
      input.value = c.text;
      input.addEventListener("change", () => {
        studentStep(core.student_answer(setJson, studentState, q.index, c.index));
      });
      label.append(input, " " + c.text);
      box.append(label);
    }
    item.append(box);
    list.append(item);
  }
  builtQuestions = true;
}

function drawStudent(view) {
  if (!builtQuestions) buildQuestions(view);
  for (const q of view.questions) {
    const inputs = $("question-" + q.id).querySelectorAll("input");
    q.choices.forEach((c, i) => {
      inputs[i].checked = c.selected;
      inputs[i].disabled = q.locked;
    });
  }
  $("progress").textContent = "Answered " + view.answered + " of " + view.total + ".";
  $("check-button").disabled = !view.can_check;
  $("scores").hidden = !view.checked;
  const body = $("score-table").querySelector("tbody");
  body.replaceChildren();
  for (const s of view.scores) {
    const row = el("tr");
    row.append(el("td", s.name), el("td", s.correct + " of " + s.attempts), el("td", s.percent + "%"));
    body.append(row);
  }
  $("overall").textContent = view.overall
    ? "All topics: " + view.overall.correct + " of " + view.overall.attempts + " (" + view.overall.percent + "%)."
    : "";
  $("save-button").disabled = !view.can_save;
}

function studentStep(text) {
  const reply = JSON.parse(text);
  if (reply.error) {
    $("student-message").textContent = reply.error;
    return;
  }
  studentState = JSON.stringify(reply.ok.state);
  $("student-message").textContent = "";
  drawStudent(reply.ok.view);
}

function saveResult() {
  const reply = JSON.parse(core.student_save(setJson, studentState, $("nickname").value));
  if (reply.error) {
    $("save-message").textContent = "";
    $("student-message").textContent = reply.error;
    return;
  }
  $("student-message").textContent = "";
  if (lastDownloadUrl) URL.revokeObjectURL(lastDownloadUrl);
  lastDownloadUrl = URL.createObjectURL(new Blob([reply.ok.contents], { type: "application/json" }));
  const link = el("a");
  link.href = lastDownloadUrl;
  link.download = reply.ok.file_name;
  document.body.append(link);
  link.click();
  link.remove();
  $("save-message").textContent = "Saved " + reply.ok.file_name + ". Hand this file to your teacher.";
}

function restart() {
  $("save-message").textContent = "";
  studentStep(core.student_start(setJson));
}

// ---------- teacher view ----------

function drawReport(report) {
  $("threshold-message").textContent = report.threshold_error || "";
  $("report-summary").textContent =
    "Files counted: " + report.counted + ". Files not counted: " + report.not_counted + "." +
    (report.reteach_topics.length ? " Suggest reteaching: " + report.reteach_topics.join(", ") + "." : "");

  const body = $("topic-table").querySelector("tbody");
  body.replaceChildren();
  for (const t of report.topics) {
    const row = el("tr");
    row.id = "topic-" + t.id;
    const share = t.percent === null ? "no data" : t.percent + "%";
    let suggestion = "—";
    if (t.reteach === true) suggestion = "Reteach (below " + report.threshold + "%)";
    if (t.reteach === false) suggestion = "At or above " + report.threshold + "%";
    row.append(
      el("td", t.name),
      el("td", share),
      el("td", t.correct + " of " + t.attempts),
      el("td", suggestion, t.reteach === true ? "reteach" : ""),
    );
    body.append(row);
  }

  const missed = $("missed");
  missed.replaceChildren();
  for (const t of report.topics) {
    const block = el("div");
    block.id = "missed-" + t.id;
    block.append(el("h4", t.name));
    const list = el("ol");
    for (const q of t.questions) {
      const item = el("li");
      item.value = q.number;
      item.append(
        el("span", q.prompt + " "),
        el("span", "(answer: " + q.answer + ") ", "muted"),
        el("span", "missed by " + q.missed + " of " + q.attempts),
      );
      if (q.most_missed) item.append(el("span", "most missed", "badge"));
      list.append(item);
    }
    block.append(list);
    missed.append(block);
  }

  $("notices-box").hidden = !report.notices.length;
  $("notices").replaceChildren(...report.notices.map((n) => el("li", n)));
  $("rejected-box").hidden = !report.rejected.length;
  $("rejected").replaceChildren(...report.rejected.map((r) => el("li", r.name + ": " + r.reason)));
}

async function refreshReport() {
  const mine = ++reportSeq;
  const files = [];
  for (const file of $("files").files) {
    files.push({ name: file.name, text: await file.text() });
  }
  if (mine !== reportSeq) return;
  const reply = JSON.parse(core.teacher_report(setJson, JSON.stringify(files), $("threshold").value));
  if (reply.error) {
    $("report-summary").textContent = reply.error;
    return;
  }
  drawReport(reply.ok);
}

// ---------- start ----------

// Whether this browser can start WebAssembly at all. A school policy that
// blocks the JavaScript JIT (Chrome's DefaultJavaScriptJitSetting) removes
// WebAssembly or makes compiling fail. The probe compiles the smallest valid
// module, the 8-byte header, so a missing or damaged pkg file is not mistaken
// for a blocked browser.
async function webAssemblyWorks() {
  if (typeof WebAssembly === "undefined") return false;
  try {
    await WebAssembly.compile(new TextEncoder().encode("\0asm\x01\0\0\0"));
    return true;
  } catch (e) {
    return false;
  }
}

async function main() {
  if (!(await webAssemblyWorks())) {
    $("load-status").hidden = true;
    $("wasm-blocked").hidden = false;
    document.querySelector("nav").hidden = true;
    document.querySelector("main").hidden = true;
    return;
  }
  try {
    await init();
    const response = await fetch("data/practice-set.json", { cache: "no-store" });
    if (!response.ok) throw new Error("the practice set did not load (" + response.status + ")");
    setJson = await response.text();
  } catch (e) {
    $("load-status").textContent = "The tool could not start: " + e.message + ".";
    return;
  }
  const start = JSON.parse(core.teacher_start(setJson));
  if (start.error) {
    $("load-status").textContent = start.error;
    return;
  }
  $("set-title").textContent = start.ok.title;
  document.title = start.ok.title;
  if (start.ok.sample) {
    $("sample-banner").textContent = "SAMPLE CONTENT. " + start.ok.note;
    $("sample-banner").hidden = false;
  }
  $("threshold").value = start.ok.threshold;

  $("tab-student").addEventListener("click", () => showView("student"));
  $("tab-teacher").addEventListener("click", () => showView("teacher"));
  $("check-button").addEventListener("click", () => studentStep(core.student_check(setJson, studentState)));
  $("restart-button").addEventListener("click", restart);
  $("save-button").addEventListener("click", saveResult);
  $("files").addEventListener("change", refreshReport);
  $("threshold").addEventListener("input", refreshReport);

  studentStep(core.student_start(setJson));
  await refreshReport();
  $("load-status").textContent = "Ready. Everything runs on this device.";
  document.body.dataset.ready = "true";
}

main();
