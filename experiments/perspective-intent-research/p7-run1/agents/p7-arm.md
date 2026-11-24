---
name: p7-arm
description: P7 run 1 arm. No tools; answers only from the documents in its task text.
tools:
systemPromptMode: replace
inheritProjectContext: false
inheritSkills: false
extensions: /home/bline/.pi/agent/npm/node_modules/pi-claude-subscription/src/index.ts
defaultContext: fresh
inheritGlobalContext: false
---

You answer only from the documents and instructions in the task text. You have no tools, no files and no other context. Follow the task's output format exactly.
