# Rules

RuleSift ships these built-in rules (`rules/builtin/*.yaml`) and three lock checks. `rulesift rules show ID` prints a rule's matcher and examples in the terminal.

| ID | Severity | Title |
|---|---|---|
| [RS-CFG-001](RS-CFG-001.md) | high | Instructions to modify agent config |
| [RS-CFG-002](RS-CFG-002.md) | high | Hook command with network or execution risk |
| [RS-CFG-003](RS-CFG-003.md) | medium | Overly broad permissions |
| [RS-CFG-004](RS-CFG-004.md) | low | Broad allowed-tools in a skill or command |
| [RS-CFG-005](RS-CFG-005.md) | medium | Remote instruction loading |
| [RS-EXEC-001](RS-EXEC-001.md) | critical | Download-and-execute script |
| [RS-EXEC-002](RS-EXEC-002.md) | critical | Decode-and-execute |
| [RS-EXEC-003](RS-EXEC-003.md) | high | Password-protected archive extracted then executed |
| [RS-EXEC-004](RS-EXEC-004.md) | high | Destructive command |
| [RS-EXEC-005](RS-EXEC-005.md) | high | Persistence mechanism |
| [RS-EXF-001](RS-EXF-001.md) | high | Image or link URL with dynamic data |
| [RS-EXF-002](RS-EXF-002.md) | high | Sends file or environment data over the network |
| [RS-EXF-003](RS-EXF-003.md) | medium | Known exfiltration, tunnel, or paste domain |
| [RS-FMT-001](RS-FMT-001.md) | medium | Missing or unparseable SKILL.md frontmatter |
| [RS-FMT-002](RS-FMT-002.md) | medium | Invalid skill name |
| [RS-FMT-003](RS-FMT-003.md) | low | Skill name does not match its directory |
| [RS-FMT-004](RS-FMT-004.md) | low | Missing, empty, or over-long skill description |
| [RS-FMT-005](RS-FMT-005.md) | low | Unknown skill frontmatter field or over-long compatibility |
| [RS-FMT-006](RS-FMT-006.md) | low | Invalid frontmatter for the tool's dialect |
| [RS-FMT-007](RS-FMT-007.md) | medium | Invalid agent config file |
| [RS-HID-001](RS-HID-001.md) | high | Instruction-like text inside an HTML comment |
| [RS-HID-002](RS-HID-002.md) | high | Base64 payload decodes to commands or URLs |
| [RS-INJ-001](RS-INJ-001.md) | high | Instruction override |
| [RS-INJ-002](RS-INJ-002.md) | high | Concealment from the user |
| [RS-INJ-003](RS-INJ-003.md) | high | Safety bypass or jailbreak persona |
| [RS-INJ-004](RS-INJ-004.md) | medium | System or role impersonation |
| [RS-LOCK-001](RS-LOCK-001.md) | high | Locked agent file or MCP server changed |
| [RS-LOCK-002](RS-LOCK-002.md) | medium | New agent file not in lock |
| [RS-LOCK-003](RS-LOCK-003.md) | low | Locked agent file missing |
| [RS-MCP-001](RS-MCP-001.md) | high | MCP server command runs a shell |
| [RS-MCP-002](RS-MCP-002.md) | high | Literal secret in MCP env or headers |
| [RS-MCP-003](RS-MCP-003.md) | high | Unpinned package runner |
| [RS-MCP-004](RS-MCP-004.md) | medium | Remote MCP server over plain HTTP |
| [RS-MCP-005](RS-MCP-005.md) | high | Instruction-like text in an MCP description (tool poisoning) |
| [RS-MCP-006](RS-MCP-006.md) | medium | MCP server auto-trusted or tools auto-approved |
| [RS-SEC-001](RS-SEC-001.md) | high | Hardcoded secret |
| [RS-SEC-002](RS-SEC-002.md) | high | Reads credential locations |
| [RS-SEC-003](RS-SEC-003.md) | medium | Dumps the environment |
| [RS-UNI-001](RS-UNI-001.md) | critical | Hidden Unicode Tag characters |
| [RS-UNI-002](RS-UNI-002.md) | high | Bidirectional control characters |
| [RS-UNI-003](RS-UNI-003.md) | medium | Zero-width characters inside words or runs |
| [RS-UNI-004](RS-UNI-004.md) | medium | Variation selectors or invisible format characters in runs |
