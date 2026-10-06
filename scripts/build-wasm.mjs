// Builds crates/core-wasm into packages/core-wasm/pkg (wasm-bindgen --target web).
// Needs the wasm-bindgen CLI at the exact version pinned in crates/core-wasm/Cargo.toml:
// on PATH, or pointed to by WASM_BINDGEN.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, stdio: 'inherit' });

const pinned = readFileSync(join(root, 'crates/core-wasm/Cargo.toml'), 'utf8').match(/wasm-bindgen = "=([\d.]+)"/)[1];
const bindgen = process.env.WASM_BINDGEN || 'wasm-bindgen';
const installed = execFileSync(bindgen, ['--version'], { encoding: 'utf8' }).trim().split(' ')[1];
if (installed !== pinned) {
  console.error(`wasm-bindgen CLI is ${installed}, but the crate pins ${pinned}. Install: cargo install wasm-bindgen-cli --version ${pinned} --locked`);
  process.exit(1);
}

run('cargo', ['build', '-p', 'scytale-core-wasm', '--target', 'wasm32-unknown-unknown', '--release', '--locked']);
run(bindgen, [
  'target/wasm32-unknown-unknown/release/scytale_core_wasm.wasm',
  '--out-dir',
  'packages/core-wasm/pkg',
  '--target',
  'web',
  '--omit-default-module-path',
]);
