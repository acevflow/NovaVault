# [`ui/tsconfig.node.json`](../../../ui/tsconfig.node.json)

## Purpose

The TypeScript configuration for the two tool configuration files that run in Node: [`vite.config.ts`](vite.config.ts.md) and [`eslint.config.js`](eslint.config.js.md). A separate project gives them Node types instead of DOM types.

## Public API

| Key                        | Type     | Value                                             | Purpose                                      |
| -------------------------- | -------- | ------------------------------------------------- | -------------------------------------------- |
| `tsBuildInfoFile`          | string   | `"./node_modules/.tmp/tsconfig.node.tsbuildinfo"` | Incremental build cache.                     |
| `target`                   | string   | `"ES2023"`                                        | Language level.                              |
| `lib`                      | string[] | `["ES2023"]`                                      | No DOM types.                                |
| `module`                   | string   | `"ESNext"`                                        | Module syntax.                               |
| `moduleResolution`         | string   | `"bundler"`                                       | Import resolution.                           |
| `types`                    | string[] | `["node"]`                                        | Node globals, such as `import.meta.dirname`. |
| `skipLibCheck`             | boolean  | `true`                                            | Skips dependency `.d.ts` files.              |
| `noEmit`                   | boolean  | `true`                                            | Type-check only.                             |
| `isolatedModules`          | boolean  | `true`                                            | Per-file compilation.                        |
| `moduleDetection`          | string   | `"force"`                                         | Every file is a module.                      |
| `verbatimModuleSyntax`     | boolean  | `true`                                            | Explicit `import type`.                      |
| `strict`                   | boolean  | `true`                                            | All strict checks.                           |
| `noUncheckedIndexedAccess` | boolean  | `true`                                            | Index access may be `undefined`.             |
| `noUnusedLocals`           | boolean  | `true`                                            | Unused locals are errors.                    |
| `noUnusedParameters`       | boolean  | `true`                                            | Unused parameters are errors.                |
| `include`                  | string[] | `["vite.config.ts", "eslint.config.js"]`          | Files in this project.                       |

## Details

### `include`

- `eslint.config.js` is listed so typescript-eslint's `projectService` can find a project for it. Because `allowJs` and `checkJs` are not set, `tsc` does not type-check it.

## Dependencies

- **Internal**: referenced by [`tsconfig.json`](tsconfig.json.md).
- **External**: `typescript`, `@types/node`.

## Used by

- `tsc -b` through [`tsconfig.json`](tsconfig.json.md).

## Tests

Not applicable: configuration file.

## Notes

- This project omits four checks that [`tsconfig.app.json`](tsconfig.app.json.md) sets: `noImplicitOverride`, `noImplicitReturns`, `noFallthroughCasesInSwitch` and `exactOptionalPropertyTypes`. The source does not say whether this is deliberate.
