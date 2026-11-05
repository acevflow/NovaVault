# [`ui/tsconfig.app.json`](../../../ui/tsconfig.app.json)

## Purpose

The TypeScript configuration for the browser code: everything under `ui/src/`, including components, logic and tests. It only type-checks (Vite transpiles), and it turns on strict mode plus extra checks, as the project rules require.

## Public API

| Key                          | Type     | Value                                                 | Purpose                                                               |
| ---------------------------- | -------- | ----------------------------------------------------- | --------------------------------------------------------------------- |
| `tsBuildInfoFile`            | string   | `"./node_modules/.tmp/tsconfig.app.tsbuildinfo"`      | Incremental build cache, kept out of the source tree.                 |
| `target`                     | string   | `"ES2022"`                                            | Language level.                                                       |
| `lib`                        | string[] | `["ES2022", "DOM", "DOM.Iterable"]`                   | Available built-in types.                                             |
| `module`                     | string   | `"ESNext"`                                            | Module syntax.                                                        |
| `moduleResolution`           | string   | `"bundler"`                                           | Resolves imports the way Vite does.                                   |
| `jsx`                        | string   | `"react-jsx"`                                         | Automatic JSX runtime.                                                |
| `types`                      | string[] | `["vite/client", "@testing-library/jest-dom/vitest"]` | Global types: Vite client and the jest-dom matchers on Vitest's `expect`. |
| `skipLibCheck`               | boolean  | `true`                                                | Skips dependency `.d.ts` files.                                       |
| `noEmit`                     | boolean  | `true`                                                | Type-check only.                                                      |
| `isolatedModules`            | boolean  | `true`                                                | Each file must compile on its own, as Vite requires.                  |
| `moduleDetection`            | string   | `"force"`                                             | Treats every file as a module.                                        |
| `verbatimModuleSyntax`       | boolean  | `true`                                                | Type-only imports must use `import type`.                             |
| `strict`                     | boolean  | `true`                                                | All strict checks.                                                    |
| `noUncheckedIndexedAccess`   | boolean  | `true`                                                | Index access may be `undefined`.                                      |
| `noImplicitOverride`         | boolean  | `true`                                                | Overrides need the `override` keyword.                                |
| `noImplicitReturns`          | boolean  | `true`                                                | Every code path must return.                                          |
| `noFallthroughCasesInSwitch` | boolean  | `true`                                                | No implicit `switch` fall-through.                                    |
| `noUnusedLocals`             | boolean  | `true`                                                | Unused locals are errors.                                             |
| `noUnusedParameters`         | boolean  | `true`                                                | Unused parameters are errors.                                         |
| `exactOptionalPropertyTypes` | boolean  | `true`                                                | An optional property cannot be set to `undefined` explicitly.         |
| `include`                    | string[] | `["src"]`                                             | Files in this project.                                                |

## Details

### Strictness

- `strict` plus the extra `no*` flags implement the "strict mode" rule in `CLAUDE.md`. The "no `any`" rule is enforced separately by [`eslint.config.js`](eslint.config.js.md).
- `noUncheckedIndexedAccess` forces code to handle a missing entry on every record or array lookup, for example when [`api/errors.ts`](src/api/errors.ts.md) maps error codes to messages.
- `exactOptionalPropertyTypes` means an optional field in [`api/types.ts`](src/api/types.ts.md) is either present with a value or absent; it is never an explicit `undefined`.

### Module settings

- `isolatedModules`, `verbatimModuleSyntax` and `moduleResolution: "bundler"` match how Vite transpiles each file separately.
- `types` makes `toBeInTheDocument()` and the other jest-dom matchers available in tests without an import.

## Dependencies

- **Internal**: referenced by [`tsconfig.json`](tsconfig.json.md).
- **External**: `typescript`, `vite` (client types), `@testing-library/jest-dom` (matcher types).

## Used by

- `tsc -b` through [`tsconfig.json`](tsconfig.json.md).
- [`eslint.config.js`](eslint.config.js.md): its type-aware rules find this project through `projectService`.

## Tests

Not applicable: configuration file.

## Notes

- Tests and app code share one project, so the jest-dom matcher types are visible to app code too. Nothing in the configuration stops app code from using them.
