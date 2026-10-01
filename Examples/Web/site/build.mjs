// The live demo as a static site: what GitHub Pages serves.
//
//     node Examples/Web/site/build.mjs [folder]        (default: _site)
//
// The site is the repository's own layout, cut down to what a browser
// fetches - the pages, lib/js with the core and js-dos, and the Examples
// the demo shows and puts on its DOS PC's disk - so every relative path
// in the pages stays true. The core, lib/js/owlosui-wire.wasm, is built
// first (lib/js/build.cmd, or cargo by hand: .github/workflows/pages.yml).
//
// Two things a static host cannot do, written here instead:
//   index.html         the site's front door: straight on to the demo
//   Examples/listing.json  every folder of Examples and what is in it, for
//                      the commander's examples drive - RUN.CMD's server
//                      answers ?list, a static host answers nothing
// and .nojekyll, so GitHub Pages serves every file as it is.

import { execFileSync } from 'node:child_process';
import { cpSync, mkdirSync, rmSync, statSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const out = resolve(process.argv[2] ?? '_site');

/** What the site carries, by the start of its path. */
const WANTED = ['lib/js/', 'lib/dos/', 'Examples/', 'CS_Demo.cmd', 'LICENSE', 'README.md'];
/** And what it does not: native build outputs, and this builder. */
const UNWANTED = [/\/target\//, /\/bin\//, /\/obj\//, /^Examples\/Web\/site\//];

// The repository's files - those it tracks and those not yet committed,
// but never what .gitignore leaves out.
const files = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard'], { cwd: root, encoding: 'utf8' })
  .split('\n')
  .filter(f => f && WANTED.some(w => f.startsWith(w)) && !UNWANTED.some(u => u.test(f)) && existsSync(join(root, f)));

const wasm = 'lib/js/owlosui-wire.wasm';
if (!existsSync(join(root, wasm))) {
  console.error(`${wasm} is not built: run lib\\js\\build.cmd first.`);
  process.exit(1);
}
files.push(wasm);

rmSync(out, { recursive: true, force: true });
for (const f of files) {
  mkdirSync(dirname(join(out, f)), { recursive: true });
  cpSync(join(root, f), join(out, f));
}

// Examples/listing.json: { "/": [entries], "/DOS/": [...] }, each entry
// as RUN.CMD's ?list gives it.
const listing = {};
const folder = dir => (listing[dir] ??= []);
for (const f of files.filter(f => f.startsWith('Examples/'))) {
  const parts = f.slice('Examples/'.length).split('/');
  for (let i = 0; i < parts.length; i++) {
    const dir = '/' + parts.slice(0, i).map(p => p + '/').join('');
    const name = parts[i];
    const isDir = i < parts.length - 1;
    if (folder(dir).some(e => e.name === name)) continue;
    const s = isDir ? null : statSync(join(root, f));
    folder(dir).push({ name, size: s ? s.size : 0, modified: Math.floor((s ? s.mtimeMs : Date.now()) / 1000), dir: isDir });
  }
}
for (const entries of Object.values(listing)) entries.sort((a, b) => (b.dir - a.dir) || a.name.localeCompare(b.name));
writeFileSync(join(out, 'Examples/listing.json'), JSON.stringify(listing));

writeFileSync(join(out, 'index.html'), `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>OWLOSUI - live demo</title>
<meta http-equiv="refresh" content="0; url=Examples/Web/JavaScript/05-Demo/">
</head>
<body style="background:#0000aa;color:#fff;font-family:monospace">
<p>OWLOSUI: a text-mode toolkit with one Rust core - in WebAssembly in this page, and on DOS in DOSBox beside it.</p>
<p><a style="color:#ff5" href="Examples/Web/JavaScript/05-Demo/">The demo</a> &middot;
<a style="color:#ff5" href="Examples/Web/JavaScript/">the JavaScript steps, one by one</a> &middot;
<a style="color:#ff5" href="https://github.com/KirinDenis/OWLOSUI">the source</a></p>
</body>
</html>
`);
writeFileSync(join(out, '.nojekyll'), '');

const bytes = files.reduce((n, f) => n + statSync(join(out, f)).size, 0);
console.log(`${out}: ${files.length} files, ${(bytes / 1048576).toFixed(1)} MB`);
