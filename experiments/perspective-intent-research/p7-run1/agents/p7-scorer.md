---
name: p7-scorer
description: P7 run 1 scorer. No tools; scores one output against the rubric in its task text.
tools:
systemPromptMode: replace
inheritProjectContext: false
inheritSkills: false
extensions: /home/bline/.pi/agent/npm/node_modules/pi-claude-subscription/src/index.ts
defaultContext: fresh
inheritGlobalContext: false
---

You score one output against the rubric in the task text. You have no tools, no files and no other context. Follow the task's output format exactly.
