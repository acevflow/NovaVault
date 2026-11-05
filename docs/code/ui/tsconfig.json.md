# [`ui/tsconfig.json`](../../../ui/tsconfig.json)

## Purpose

The root TypeScript configuration of the UI. It compiles nothing itself. It only references the two real projects, so `tsc -b` checks the browser sources and the Node-side tool configuration with different settings.

## Public API

| Key          | Type     | Value                                                                       | Purpose                              |
| ------------ | -------- | --------------------------------------------------------------------------- | ------------------------------------ |
| `files`      | string[] | `[]`                                                                        | No files belong to the root project. |
| `references` | object[] | `[{ "path": "./tsconfig.app.json" }, { "path": "./tsconfig.node.json" }]` | The two projects `tsc -b` builds.    |

## Details

### `references`

- [`tsconfig.app.json`](tsconfig.app.json.md): browser code under `src/`, tests included.
- [`tsconfig.node.json`](tsconfig.node.json.md): `vite.config.ts` and `eslint.config.js`, which run in Node.
- The empty `files` list stops `tsc` from picking up every `.ts` file with default settings.

## Dependencies

- **Internal**: the two referenced configurations.
- **External**: `typescript`.

## Used by

- `npm run typecheck` and `npm run build` (`tsc -b`), see [`package.json`](package.json.md).
- Editors, which use it to find the project each file belongs to.

## Tests

Not applicable: configuration file.

## Notes

- None.
