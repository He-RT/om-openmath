// Verify installed bytes/identities against the exact production lock; never run install hooks.
import { readFile, readdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
const root = fileURLToPath(new URL('..', import.meta.url));
const lock = JSON.parse(await readFile(resolve(root, 'package-lock.json'), 'utf8'));
const licenses = new Set(['MIT', 'Apache-2.0', 'BSD-3-Clause', 'Unlicense', '0BSD']);
const overrides = JSON.parse(await readFile(resolve(root, 'notice-overrides.json'), 'utf8')).packages;
const rows = [];
for (const [path, metadata] of Object.entries(lock.packages)) {
  if (!path || metadata.dev) continue;
  const folder = resolve(root, path);
  const pkg = JSON.parse(await readFile(resolve(folder, 'package.json'), 'utf8'));
  if (pkg.version !== metadata.version || !licenses.has(metadata.license)) {
    throw new Error(`Unverified package identity/license: ${path}`);
  }
  const notices = [];
  for (const name of await readdir(folder)) {
    if (!/^(licen[cs]e|copying|notice|unlicense)(\.|$)/i.test(name)) continue;
    const bytes = await readFile(resolve(folder, name));
    notices.push({ name, sha256: createHash('sha256').update(bytes).digest('hex') });
  }
  if (!notices.length) {
    const readme = await readFile(resolve(folder, 'README.md')).catch(() => undefined);
    if (readme && /permission is hereby granted, free of charge/i.test(readme.toString('utf8'))) {
      notices.push({ name: 'README.md', sha256: createHash('sha256').update(readme).digest('hex') });
    }
  }
  if (!notices.length) {
    const source = overrides[path];
    if (!source || source.version !== pkg.version || source.integrity !== metadata.integrity || source.license !== metadata.license) {
      throw new Error(`Missing verified upstream notice: ${path}`);
    }
    const bytes = await readFile(resolve(root, '..', source.notice_path));
    if (createHash('sha256').update(bytes).digest('hex') !== source.sha256) throw new Error(`Notice hash mismatch: ${path}`);
    notices.push({ name: source.notice_path, sha256: source.sha256, source: source.upstream_url });
  }
  rows.push({ path, name: pkg.name, version: pkg.version, license: metadata.license,
    integrity: metadata.integrity, notices,
    skipped_lifecycle_scripts: Object.fromEntries(Object.entries(pkg.scripts ?? {}).filter(([name]) => /^(preinstall|install|postinstall)$/.test(name))) });
}
console.log(JSON.stringify({ schema_version: 1, kind: 'development_dependency_audit',
  release_gate_evidence: false, production_packages: rows.length,
  lock_sha256: createHash('sha256').update(await readFile(resolve(root, 'package-lock.json'))).digest('hex'), packages: rows }, null, 2));
