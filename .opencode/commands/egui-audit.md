---
description: Detect semantic and logic bugs in egui programs, translate HTML+CSS to correct egui patterns, and produce critique/polish reports with P0-P3 severity and JSON output. Forked from impeccable and upgraded for egui 0.36 + eframe native codebases ported from HTML/CSS interfaces.
agent: build
subtask: true
---

Call skill({ name: "egui-audit" }) and follow its SKILL.md to handle $ARGUMENTS. Supported verbs: detect, port, critique, report --json. Example: /egui-audit detect src/app.rs
