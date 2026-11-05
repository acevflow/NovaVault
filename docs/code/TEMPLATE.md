# Per-file code documentation template

This is the standard every page under `docs/code/` must follow. Each page documents exactly one source file and is written so that a reader can understand what the file does, how to use it and how it handles secrets without opening the code first.

## Layout

`docs/code/` mirrors the source tree. Each hand-written source file gets one Markdown page at the same relative path with `.md` appended to the full file name, extension included. The examples below illustrate the mapping rule; they are not a list of files that exist.

| Source file                                 | Documentation page                                          |
| ------------------------------------------- | ----------------------------------------------------------- |
| `crates/novavault-core/src/crypto.rs`       | `docs/code/crates/novavault-core/src/crypto.rs.md`          |
| `crates/novavault-core/src/aead.rs`         | `docs/code/crates/novavault-core/src/aead.rs.md`            |
| `src-tauri/tauri.conf.json`                 | `docs/code/src-tauri/tauri.conf.json.md`                    |
| `.github/workflows/<workflow>.yml`          | `docs/code/.github/workflows/<workflow>.yml.md`             |

### Covered

- Rust source files (`.rs`).
- TypeScript and TSX source files (`.ts`, `.tsx`).
- Significant configuration: `Cargo.toml`, `tauri.conf.json`, Tauri capability files, `vite.config.*`, `tsconfig*.json` and CI workflows.

### Excluded

- Generated files.
- Lockfiles (`Cargo.lock`, `package-lock.json`).
- Build output.
- `node_modules/` and `target/`.
- Everything under `docs/`.

### Tests

Test files get one page per test module or file. A Rust `#[cfg(test)] mod tests` inside a source file is documented in the **Tests** section of that file's page; a standalone test file (for example under `tests/`) gets its own page.

## Required sections

Every page has these sections, in this order. Omit a section only if it truly does not apply, and when you do, keep the heading and say so in one line, for example: _Not applicable: this file exposes no public items._

1. **Title**: the file path, linked to the source.
2. **Purpose**: what the file does and where it fits in the architecture, in two or three sentences.
3. **Public API**: a table of every public item with its name, kind, signature and a one-line description.
4. **Details**: one subsection per item, in source order, covering:
   - behavior and algorithm, step by step when non-trivial;
   - parameters, return values and errors, with the conditions that trigger each error;
   - security notes: what is secret, how it is zeroized, what is trusted and what is not;
   - edge cases and invariants.
5. **Dependencies**: internal modules used, and external crates or packages with the reason each one is needed.
6. **Used by**: which files call or import it.
7. **Tests**: which tests cover it and what they verify, with links.
8. **Notes**: known limitations, design decisions and open questions.

### Section guidance by file type

- **Configuration files**: in **Public API**, list the significant keys or settings instead of code items (key, type, value, purpose). In **Details**, explain why each security-relevant setting has its value (for example CSP directives or granted capabilities).
- **Test files**: in **Public API**, list the test cases (name, what it checks). In **Tests**, link to the source files under test.
- **React components**: in **Public API**, give the props type as the signature. In **Details**, describe rendered states, user interactions and accessibility behavior.

## Accuracy rules

- Read each source file in full before documenting it.
- Verify every claim against the code.
- Copy signatures exactly as they appear in the source, including generics, lifetimes and visibility.
- Where existing documentation and code disagree, the code wins. Record the disagreement under **Notes**.
- Never invent behavior. Do not describe what a function "should" do or will do in the future.
- Put anything unclear under **Notes** as an open question instead of guessing.
- Use obviously fake values in examples (for example `correct horse battery staple`, `0000-0000-0000-0000` or `example-vault.nvault`). Never include real secrets, real keys or real vault files.

## Links

All links are relative to the page's own location. A page at `docs/code/<path>/<file>.md` reaches the repository root by going up one level for `docs/`, one for `code/` and one per directory in `<path>`.

For example, `docs/code/crates/novavault-core/src/aead.rs.md` sits five directories below the root (`docs/code/crates/novavault-core/src/`), so it links to its source as `../../../../../crates/novavault-core/src/aead.rs` and to another page as `kdf.rs.md`.

## Skeleton

Copy this skeleton and replace every `<placeholder>`.

````markdown
# [`<relative/path/to/file>`](<relative link to the source file>)

## Purpose

<What the file does and where it fits in the architecture, in two or three sentences.>

## Public API

| Name     | Kind                         | Signature                 | Description          |
| -------- | ---------------------------- | ------------------------- | -------------------- |
| `<name>` | <function, struct, enum, …> | `<exact signature>`       | <One-line summary.>  |

## Details

### `<name>`

<Behavior and algorithm. Use numbered steps when non-trivial.>

- **Parameters**: <each parameter and its meaning>.
- **Returns**: <what is returned on success>.
- **Errors**:
  - `<ErrorVariant>`: <condition that triggers it>.
- **Security**: <what is secret, how it is zeroized, what is trusted>.
- **Edge cases and invariants**: <boundary inputs, ordering guarantees, invariants upheld>.

## Dependencies

- **Internal**: [`<module>`](<link to its page>): <what is used and why>.
- **External**: `<crate or package>`: <why it is needed>.

## Used by

- [`<file>`](<link to its page>): <how it uses this file>.

## Tests

- [`<test name>`](<link to the test>): <what it verifies>.

## Notes

- <Known limitation, design decision or open question.>
````
