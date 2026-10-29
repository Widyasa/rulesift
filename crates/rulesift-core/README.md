# rulesift-core

The library behind [RuleSift](https://github.com/Widyasa/rulesift), an
offline linter and PR gate for AI agent configuration files. It contains the
tool registry, discovery, parsers, rules, scoring, diff, lock, and reports
(JSON, SARIF, Markdown).

Most users want the `rulesift` CLI instead: `cargo install rulesift`. The
library API is not yet stable and may change in minor releases.

Dual-licensed under MIT OR Apache-2.0.
