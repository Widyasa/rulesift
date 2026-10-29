---
name: db-reader
description: Answers questions about the example database.
tools: Read, Grep, Bash(sqlite3:*)
disallowedTools: Write
permissionMode: default
mcpServers:
  sqlite:
    command: echo
    args: ["sqlite"]
---

Answer questions using read-only queries.
