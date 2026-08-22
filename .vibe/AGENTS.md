## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

When the user types `/graphify`, use the installed graphify skill or instructions before doing anything else.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- Dirty graphify-out/ files are expected after hooks or incremental updates; dirty graph files are not a reason to skip graphify. Only skip graphify if the task is about stale or incorrect graph output, or the user explicitly says not to use it.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).

## Coding Standards

### Code Organization
- main() must contain ONLY user interaction and action orchestration — extract all infrastructure (tracing, config parsing, etc.) to separate modules
- Group impl blocks for foreign types in the module where they are *most relevant to readers*, not where the foreign type is defined (e.g., tracing impls belong in a `_tracing.rs` module)
- Prefer single, clear initialization calls over complex multi-step registry building

### API Design
- Leverage framework features fully (e.g., use clap's ValueEnum, conflicts_with, requires, default_value_os_t) for free help text and error handling
- Use enums for flag-driven levels (e.g., -v/-q → LogLevel enum) — research framework capabilities to find the right pattern
- Always prefer `?` and Try-based error propagation over .expect() or panic
- Implement From trait *immediately* on first use — this is an exception to YAGNI

### Testing
- Always create tests for new functionality
- Tests MUST assert on the actual behavior being tested — tests that don't verify output are useless
- Tests should be concise but meaningful; they serve as executable documentation

### Tooling
- Run `cargo clippy --all-features --all-targets -- -D warnings` by default unless project Cargo.toml explicitly states otherwise
- Check for cross-compilation constraints in Cargo.toml comments before assuming native execution
