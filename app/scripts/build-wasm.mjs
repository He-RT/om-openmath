import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { mkdirSync } from 'node:fs';
const root = fileURLToPath(new URL('../../', import.meta.url));
const output = fileURLToPath(new URL('../src/kernel/wasm/', import.meta.url));
const run = (command, args) => {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit' });
  if (result.error || result.status !== 0) throw new Error(`${command} failed; install wasm-bindgen-cli 0.2.129 and the wasm32 target.`);
};
const version = spawnSync('wasm-bindgen', ['--version'], { encoding: 'utf8' });
if (version.stdout?.trim() !== 'wasm-bindgen 0.2.129') throw new Error('Expected wasm-bindgen-cli 0.2.129.');
run('cargo', ['build', '-p', 'om-wasm', '--release', '--locked', '--target', 'wasm32-unknown-unknown']);
mkdirSync(output, { recursive: true });
run('wasm-bindgen', ['--target', 'web', '--out-dir', output, `${root}/target/wasm32-unknown-unknown/release/om_wasm.wasm`]);
