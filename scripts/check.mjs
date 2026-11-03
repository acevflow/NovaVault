#!/usr/bin/env node
/**
 * Full local check for NovaVault: formatting, linting, type checking, tests
 * and builds for the UI and the Rust workspace. Run it with `npm run check`
 * from the repository root before every push. CI runs the same steps.
 *
 * The UI is built before the Rust steps because the Tauri shell embeds
 * `ui/dist` at compile time.
 *
 * Steps run in order and the script stops at the first failure.
 */
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const ui = join(root, 'ui');

/** @type {{ name: string, command: string, cwd: string }[]} */
const steps = [];

if (!existsSync(join(ui, 'node_modules'))) {
  steps.push({ name: 'UI: install dependencies', command: 'npm ci', cwd: ui });
}

steps.push(
  { name: 'UI: format', command: 'npm run format:check', cwd: ui },
  { name: 'UI: lint', command: 'npm run lint', cwd: ui },
  { name: 'UI: typecheck', command: 'npm run typecheck', cwd: ui },
  { name: 'UI: test', command: 'npm test', cwd: ui },
  { name: 'UI: build', command: 'npm run build', cwd: ui },
  { name: 'Rust: format', command: 'cargo fmt --all -- --check', cwd: root },
  {
    name: 'Rust: clippy',
    command: 'cargo clippy --workspace --all-targets --locked -- -D warnings',
    cwd: root,
  },
  { name: 'Rust: test', command: 'cargo test --workspace --locked', cwd: root },
);

for (const [index, step] of steps.entries()) {
  console.log(`\n[${index + 1}/${steps.length}] ${step.name}: ${step.command}`);
  // A shell resolves `npm` to `npm.cmd` on Windows. Commands are fixed strings
  // defined above, so no untrusted input reaches the shell.
  const result = spawnSync(step.command, { cwd: step.cwd, stdio: 'inherit', shell: true });
  if (result.status !== 0) {
    console.error(`\nCheck failed at "${step.name}".`);
    process.exit(result.status ?? 1);
  }
}

console.log('\nAll checks passed.');
