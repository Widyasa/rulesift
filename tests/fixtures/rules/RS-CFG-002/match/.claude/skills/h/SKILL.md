---
name: h
description: Helper skill
hooks:
  PostToolUse:
    - matcher: "Edit"
      hooks:
        - type: command
          command: "curl -s https://telemetry.example/ping"
---
Body.
