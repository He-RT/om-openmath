// Retain upstream license/notice texts in both static and native distributions.
import { cp, readdir, readFile, mkdir, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
const app = fileURLToPath(new URL('../', import.meta.url));
const root = dirname(app);
const dist = join(app, 'dist');
for (const name of ['LICENSE-MIT', 'LICENSE-APACHE', 'THIRD_PARTY_NOTICES.md', 'licenses']) {
  await cp(join(root, name), join(dist, name), { recursive: true });
}
const index = [];
async function retain(path, group, name, version, license, source) {
  const destination = join(dist, 'licenses', group, `${name.replaceAll('/', '__')}-${version}`);
  await mkdir(destination, { recursive: true });
  const files = (await readdir(path)).filter((file) => /^(licen[sc]e|copying|notice|unlicense)/i.test(file));
  for (const file of files) await cp(join(path, file), join(destination, file), { recursive: true });
  index.push(`${group}/${name} ${version} | ${typeof license === 'string' ? license : JSON.stringify(license ?? 'see upstream')} | ${source ?? 'see package source'} | ${files.join(', ') || 'license declared by upstream manifest'}`);
}
const cargo = JSON.parse(execFileSync('cargo', ['metadata', '--locked', '--format-version', '1'], { cwd: root, encoding: 'utf8', maxBuffer: 20 * 1024 * 1024 }));
for (const pkg of cargo.packages.filter((pkg) => pkg.source)) {
  await retain(dirname(pkg.manifest_path), 'rust', pkg.name, pkg.version, pkg.license, `https://crates.io/crates/${pkg.name}/${pkg.version}`);
}
const modules = join(app, 'node_modules');
for (const entry of await readdir(modules, { withFileTypes: true })) {
  if (!entry.isDirectory() || entry.name.startsWith('.')) continue;
  const paths = entry.name.startsWith('@') ? (await readdir(join(modules, entry.name))).map((child) => join(modules, entry.name, child)) : [join(modules, entry.name)];
  for (const path of paths) {
    let pkg;
    try { pkg = JSON.parse(await readFile(join(path, 'package.json'), 'utf8')); } catch { continue; }
    await retain(path, 'npm', pkg.name, pkg.version, pkg.license, pkg.repository?.url ?? pkg.homepage);
  }
}
await writeFile(join(dist, 'licenses', 'INDEX.txt'), index.sort().join('\n') + '\n');
console.log(`Packaged upstream notices for ${index.length} locked packages.`);
