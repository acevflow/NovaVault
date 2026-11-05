# [`src-tauri/src/main.rs`](../../../../src-tauri/src/main.rs#L127) (tests)

## Purpose

The inline `#[cfg(test)] mod tests` of `main.rs` guards the IPC surface: it checks that the window capability grants exactly the commands that `build.rs` declares, no more and no fewer, so a newly added command is never callable by accident and a declared one is never silently unreachable.

## Public API

| Name                                            | Kind | Signature                                            | Description                                                         |
| ----------------------------------------------- | ---- | ---------------------------------------------------- | ------------------------------------------------------------------- |
| `capability_grants_exactly_the_declared_commands` | test | `fn capability_grants_exactly_the_declared_commands()` | `build.rs` `COMMANDS` and `capabilities/default.json` permissions match. |

## Details

### [`capability_grants_exactly_the_declared_commands`](../../../../src-tauri/src/main.rs#L132)

1. Embeds [`build.rs`](../../../../src-tauri/build.rs) with `include_str!`, takes the text from `const COMMANDS` up to the first `];`, and keeps every second `"`-delimited piece, which yields the quoted command names.
2. Maps each name to Tauri's generated permission identifier, `allow-<name with _ replaced by ->`, and sorts the list.
3. Embeds [`capabilities/default.json`](../../../../src-tauri/capabilities/default.json), parses it with `serde_json` and collects its `permissions` array as strings, sorted.
4. Asserts that 19 commands were declared and that the two lists are equal.

Because both files are embedded at compile time, editing either one rebuilds the test.

## Dependencies

- **Internal**: the source text of [`build.rs`](../../../../src-tauri/build.rs) (see [`build.rs.md`](../build.rs.md)) and [`capabilities/default.json`](../../../../src-tauri/capabilities/default.json) (see [`default.json.md`](../capabilities/default.json.md)).
- **External**: `serde_json`: parses the capability file.

## Used by

_Not applicable: test module, run by `cargo test`._

## Tests

Code under test: the command list in [`src-tauri/build.rs`](../../../../src-tauri/build.rs) and the capability [`src-tauri/capabilities/default.json`](../../../../src-tauri/capabilities/default.json). The module lives in [`main.rs`](main.rs.md) because that is the binary's crate root.

How to run:

```sh
cargo test -p novavault tests::capability_grants_exactly_the_declared_commands
```

The full local check (`npm run check`, see [`scripts/check.mjs`](../../../../scripts/check.mjs)) and CI (`cargo test --workspace --locked`) run it too.

## Notes

- The parser is textual: a comment containing a `"` or a `];` inside `COMMANDS` would break it. The current list has neither.
- The hard-coded count of 19 must be updated whenever a command is added or removed, which is intentional friction.
- The test does not compare against `generate_handler!` in `main.rs`; a command declared and granted but not registered fails only at runtime (see [`main.rs.md`](main.rs.md)).
- Granting a core or plugin permission (for example `dialog:default`) in the capability would also fail this test, since it requires an exact match.
