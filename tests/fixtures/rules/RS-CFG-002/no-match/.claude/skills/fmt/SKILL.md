---
name: fmt
description: Formats code after edits.
hooks:
  PostToolUse:
    - matcher: "Edit|Write"
      hooks:
        - type: command
          command: npx prettier --write .
---
Format files.
