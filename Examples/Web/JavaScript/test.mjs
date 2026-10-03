// The web examples, driven without a browser: Node loads the same core
// (lib/js/owlosui-wire.wasm) and the same client (lib/js/owlosui.js) the
// pages load, builds each example's App, presses its keys and clicks its
// buttons by the wire, and reads the frame as text. What the pages show is
// what these frames hold - the canvas only paints them.
//
//     node Examples/Web/JavaScript/test.mjs
//
// Exits 1 if any case failed. No test framework: a line per case.

import { readFileSync } from 'node:fs';
import { Wire, Owlosui, Style } from '../../../lib/js/owlosui.js';
import * as Hello from './01-HelloWorld/app.js';
import * as Notes from './02-Notes/app.js';
import * as Calc from './03-Calculator/app.js';
import { CalcEngine, Base } from './03-Calculator/calc-engine.js';
import * as Soko from './04-Sokoban/app.js';
import * as Demo from './05-Demo/app.js';
import { Board } from './05-Demo/tools.js';
import { log } from './05-Demo/log.js';

const bytes = readFileSync(new URL('../../../lib/js/owlosui-wire.wasm', import.meta.url));
let failed = 0;

/** A fresh core for every case: one module, one screen. */
async function owl(w = 80, h = 25) {
  return new Owlosui(await Wire.load(bytes), w, h);
}

async function test(name, body) {
  try {
    await body();
    console.log('ok    ' + name);
  } catch (e) {
    failed++;
    console.log('FAIL  ' + name + '\n      ' + String(e.message).split('\n').join('\n      '));
  }
}

function check(ok, what, frame) {
  if (!ok) throw new Error(what + (frame ? '\n' + frame.toString() : ''));
}

/** Press a key, let a held button happen, and hand whatever it caused to the app. */
function key(o, app, k, mods) {
  o.press(k, mods);
  o.tick();
  const { pressed, command } = o.take();
  let going = true;
  if (pressed) going = app.onCommand(pressed) !== false && going;
  if (command) going = app.onCommand(command) !== false && going;
  app.poll?.();
  return going;
}

function click(o, app, x, y) {
  o.click(x, y);
  const { pressed, command } = o.take();
  if (pressed) app.onCommand(pressed);
  if (command) app.onCommand(command);
  app.poll?.();
}

// ------------------------------------------------------------ 1 HelloWorld

await test('HelloWorld: the words are on the screen, and Enter ends it', async () => {
  const o = await owl();
  const app = new Hello.App(o);
  const f = o.frame();
  check(f.find('Hello, world!') && f.find(' OK '), 'no window', f);
  check(key(o, app, 'Enter') === false, 'Enter did not end the program');
});

// ----------------------------------------------------------------- 2 Notes

await test('Notes: typing, Save keeps it; Exit with changes asks, Stay stays', async () => {
  const o = await owl();
  const store = new Map();
  const storage = { getItem: k => store.get(k) ?? null, setItem: (k, v) => store.set(k, v) };
  const app = new Notes.App(o, storage);
  o.type('hello web');
  check(app.modified, 'typing did not change the text');
  key(o, app, 's', { alt: true });
  check(store.get('owlosui:NOTES.TXT') === 'hello web', `Save wrote ${store.get('owlosui:NOTES.TXT')}`);
  o.type('!');
  check(key(o, app, 'x', { alt: true }) !== false, 'Exit with changes did not ask');
  // The question wraps: look for its first words.
  check(o.frame().find('Leave without'), 'no question', o.frame());
  key(o, app, 'Enter');
  check(!o.frame().find('Leave without'), 'Stay did not close the question', o.frame());
  // Reopened: the saved text is back.
  const again = new Notes.App(await owl(), storage);
  check(again.owl.getText(again.editor) === 'hello web', 'the text did not survive');
});

await test('Notes: Find selects, Replace all replaces', async () => {
  const o = await owl();
  const app = new Notes.App(o, null);
  o.type('cat and cat');
  key(o, app, 'r', { alt: true });
  check(o.frame().find('Replace [modal]'), 'no Replace dialog', o.frame());
  o.type('cat');
  o.press('Tab');
  o.type('dog');
  key(o, app, 'a', { alt: true });
  check(o.frame().find('2 replaced'), 'Replace all did not say how many', o.frame());
  check(o.getText(app.editor) === 'dog and dog', `replaced wrongly: ${o.getText(app.editor)}`);
});

// ------------------------------------------------------------ 3 Calculator

await test('Calculator engine: precedence, functions, bases and errors', async () => {
  const c = new CalcEngine();
  const is = (expr, want) => { const got = c.evaluate(expr); check(got === want, `${expr} = ${got}, wanted ${want}`); };
  is('1+2*3', '7'); is('(1+2)*3', '9'); is('10/4', '2.5'); is('2^3^2', '512'); is('-3^2', '-9'); is('2^-1', '0.5');
  is('17 mod 5', '2'); is('17%5', '2'); is('5!', '120'); is('sqrt(2)', '1.4142135623731'); is('sqrt(16)+1', '5');
  is('pi', '3.14159265358979'); is('e', '2.71828182845905'); is('ln(e)', '1'); is('log(1000)', '3');
  is('0.1+0.2', '0.3'); is('1/3', '0.333333333333333'); is('2^70', '1.18059162071741E+21');
  is('sin(30)', '0.5'); is('cos(60)', '0.5'); is('tan(45)', '1'); is('sin(180)', '0'); is('asin(1)', '90');
  c.degrees = false; is('sin(pi/2)', '1'); is('cos(pi)', '-1'); c.degrees = true;
  is('1/0', 'Divide by zero'); is('5 mod 0', 'Divide by zero'); is('7-', 'Error'); is('(1+2', 'Error'); is('1 2', 'Error');
  is('foo(1)', 'Error'); is('sqrt(-1)', 'Invalid input'); is('(-1)!', 'Invalid input'); is('200!', 'Overflow'); is('10^400', 'Overflow');
  is('', ''); is('1e', 'Error');
  c.base = Base.Hex; is('FF+1', '100'); is('ace', 'ACE'); is('10/4', '4'); is('-A', '-A'); is('1.5', 'Invalid input');
  is('FFFFFFFFFFFFFFFFFF', 'Overflow');
  c.base = Base.Bin; is('1010+1', '1011'); is('2', 'Error');
  c.base = Base.Oct; is('17+1', '20'); is('8', 'Error');
  c.base = Base.Dec; c.memory = 42; is('m+1', '43');
});

await test('Calculator: the keypad is buttons, C is red, typing goes to the display, Enter is =', async () => {
  const o = await owl();
  const app = new Calc.App(o);
  const f = o.frame();
  const c = f.find('  C  ');
  check(c, 'no C key', f);
  check(f.attr(c.x + 2, c.y) === 0x4F, `C is ${f.attr(c.x + 2, c.y).toString(16)}, not white on red`, f);
  o.type('12+34*2');
  key(o, app, 'Enter');
  check(app.calc.text === '80', `12+34*2 = ${app.calc.text}`);
  // A click on 7 keeps the caret in the display.
  const seven = o.frame().find('  7  ');
  key(o, app, 'Escape');
  click(o, app, seven.x + 2, seven.y);
  o.type('+8');
  key(o, app, 'Enter');
  check(app.calc.text === '15', `7+8 = ${app.calc.text}`);
});

await test('Calculator: Hex converts the display and enables A to F', async () => {
  const o = await owl();
  const app = new Calc.App(o);
  o.type('255');
  const hex = o.frame().find('Hex');
  click(o, app, hex.x - 3, hex.y);
  check(app.calc.text === 'FF', `255 in hex: ${app.calc.text}`, o.frame());
  const a = o.frame().find('  A  ');
  check(o.frame().attr(a.x + 2, a.y) === 0x20, 'A is not enabled in hex', o.frame());
});

// --------------------------------------------------------------- 4 Sokoban

await test('Sokoban: the list, Enter plays level 1, an arrow moves the keeper', async () => {
  const o = await owl();
  const app = new Soko.App(o);
  check(o.frame().find('Level  1'), 'no list of levels', o.frame());
  key(o, app, 'Enter');
  // A static line folds runs of spaces into one.
  check(o.frame().find('Level 1 moves 0'), 'the game did not start', o.frame());
  // Level 1: the keeper can go up.
  key(o, app, 'ArrowUp');
  check(o.frame().find('moves 1'), 'the arrow did not move the keeper', o.frame());
  key(o, app, 'z');
  check(o.frame().find('moves 0'), 'Z did not take the step back', o.frame());
  key(o, app, 'Escape');
  check(o.frame().find('Level  1'), 'Escape did not return to the list', o.frame());
});

// ------------------------------------------------------------------ 5 Demo

await test('Demo: menu bar, hints, and every tool opens', async () => {
  const o = await owl();
  const app = new Demo.App(o);
  let f = o.frame();
  check(f.row(0).includes('File') && f.row(0).includes('Tools') && f.row(24).includes('F1 Help'), 'no bars', f);
  o.press('t', { alt: true });
  f = o.frame();
  check(f.find('Calculator') && f.find('Commander') && f.row(24).includes('Two panels of files'), 'Tools did not open with its hint', f);
  o.press('Escape');
  // A tool opens in front, so its title is on the screen the moment it opens.
  const tools = [[Demo.CmCalc, 'Calculator'], [Demo.CmCalendar, 'Calendar'], [Demo.CmAscii, 'ASCII table'], [Demo.CmPuzzle, 'Puzzle']];
  for (const [cmd, title] of tools) {
    app.onCommand(cmd);
    f = o.frame();
    check(f.find(` ${title} `), `no ${title} window`, f);
  }
  // Close the front one, and the list of windows knows.
  app.onCommand(Demo.CmClose);
  o.press('0', { alt: true });
  check(o.frame().find('Windows [modal]'), 'Alt+0 did not list the windows', o.frame());
});

await test('Demo: Tools > Console shows what the page did, coloured, and gives it back as plain text', async () => {
  const o = await owl(100, 30);
  const app = new Demo.App(o);
  log.error('error', 'something broke before anyone looked');
  o.press('t', { alt: true });
  check(o.frame().find('Console'), 'Tools has no Console', o.frame());
  o.press('Escape');
  app.onCommand(Demo.CmConsole);
  let f = o.frame();
  check(f.find(' Console ') && f.find('the core is running'), 'the console does not show the record from before it opened', f);
  const at = f.find('something broke');
  check(at && f.attr(at.x, at.y) === 0x0C, `an error is not bright red: ${at && f.attr(at.x, at.y).toString(16)}`, f);
  // A line written while it is open arrives a moment later, at the bottom.
  log.info('dos', 'switching on');
  await new Promise(r => setTimeout(r, 10));
  f = o.frame();
  check(f.find('switching on'), 'a new line did not arrive', f);
  const text = o.getText(app.console.view);
  check(text.includes('something broke before anyone looked') && !text.includes('\x1b'), 'the text is not the record without its colours');
});

await test('A tree opens a lazy node by asking, a list gives its marks, a window can go without [\u2193]', async () => {
  const o = await owl();
  const w = o.window('Folders', 40, 12);
  const t = o.tree(w, [{ text: 'C:', open: true, children: [{ text: 'DOS', lazy: true }, { text: 'GAMES' }] }]);
  let f = o.frame();
  check(f.find('DOS') && f.find('GAMES'), 'the tree is not drawn', f);
  // Down to DOS, Right opens it: the lazy node is asked for, once.
  o.press('ArrowDown');
  o.press('ArrowRight');
  const asked = o.treeExpand(t);
  check(asked && asked.path.join() === '0,0' && asked.texts.join('\\') === 'C:\\DOS', `the wrong node was asked for: ${JSON.stringify(asked)}`);
  check(o.treeExpand(t) === null, 'the same node was asked for twice');
  o.treeChildren(t, asked.path, [{ text: 'UTILS' }]);
  o.press('ArrowDown');
  check(o.treePath(t).join('\\') === 'C:\\DOS\\UTILS', `the path is ${o.treePath(t).join('\\')}`);

  const d = o.window('Pick', 30, 10, { style: Style.Dialog });
  const l = o.list(d, 1, 1, 26, 5, ['one', 'two', 'three'], { multi: true });
  o.focus(l);
  o.press('Insert');
  o.press('Insert');
  check(o.marked(l).join() === '0,1', `marked: ${o.marked(l).join()}`);

  const plain = o.window('Plain', 40, 6, { minimize: false });
  check(!o.frame().find('[\u2193]'), 'a window made with minimize: false has a minimize box', o.frame());
  o.close(plain);
  o.window('With', 40, 6);
  check(o.frame().find('[\u2193]'), 'an ordinary window has no minimize box', o.frame());
});

await test('A file of 200 KB opens whole, grows, and comes back to the last character', async () => {
  // A request carries 64 KB; a lesson's source is three times that. The
  // text goes in in pieces and comes out in parts, and a text that grew
  // past 64 KB in the editor is read back whole - it used to come back as
  // the few hundred bytes its wrapped length said, and Save wrote those.
  const o = await owl();
  const line = i => `        mov ax,${String(i).padStart(5)}    ; строка ${i}, \u{1F989} owl`;
  const big = Array.from({ length: 4000 }, (_, i) => line(i)).join('\n');
  check(new TextEncoder().encode(big).length > 180000, 'the text is not big enough to test anything');
  const w = o.window('BIG.ASM', 60, 16);
  const t = o.text(w, big);
  check(o.getText(t) === big, 'the text did not come back as it went in');
  check(o.syntax(t, 'BIG.ASM') === 'Assembler', 'not coloured as assembler');
  o.press('End', { ctrl: true });
  for (let i = 0; i < 300; i++) o.press('x');
  check(o.getText(t) === big + 'x'.repeat(300), 'what was typed at the end is not all there');
  o.setText(t, big + '\nreplaced');
  check(o.getText(t) === big + '\nreplaced', 'setText of a big text did not keep all of it');
});

await test('Demo: an editor brings an Edit menu the core answers - Word wrap, Find', async () => {
  const o = await owl();
  const app = new Demo.App(o);
  check(!o.frame().row(0).includes('Edit'), 'no editor, no Edit menu', o.frame());
  app.onCommand(Demo.CmNew);
  let f = o.frame();
  check(f.row(0).includes('File  Edit  Tools'), 'the Edit menu is not after File', f);
  // Alt+E, W: Word wrap - the core does it, the program hears nothing.
  const w = o.active();
  const t = app.docs.get(w).text;
  key(o, app, 'e', { alt: true });
  check(o.frame().find('Word wrap') && o.frame().find('Find...'), 'no editor items', o.frame());
  key(o, app, 'w');
  check(o.editorState(t).wrap, 'Word wrap did not turn on');
  // Ctrl+F: the core's own Find dialog; Escape leaves it.
  key(o, app, 'f', { ctrl: true });
  check(o.frame().find('Text to find'), 'no Find dialog', o.frame());
  key(o, app, 'Escape');
  check(o.active() === w, 'the editor is not in front again');
});

await test('Demo: a file is coloured as the language its name says', async () => {
  const o = await owl();
  const app = new Demo.App(o);
  app.addDoc('HELLO.PAS', 'begin { hi }\nend.', null, null);
  const t = app.docs.get(o.active()).text;
  check(o.editorState(t).syntax, 'no language for HELLO.PAS');
  const f = o.frame();
  const at = f.find('begin');
  check(f.attr(at.x, at.y) === 0x1F, `begin is not white on blue: ${f.attr(at.x, at.y).toString(16)}`, f);
  const c = f.find('{ hi }');
  check(f.attr(c.x, c.y) === 0x17, 'the comment is not grey', f);
  // A name nobody answers to stays plain; Edit > Syntax lists the languages.
  check(o.syntax(t, 'NOTES.XYZ') === null && !o.editorState(t).syntax, 'NOTES.XYZ got a language');
  key(o, app, 'e', { alt: true });
  key(o, app, 'y');
  check(o.frame().find('Pascal') && o.frame().find('Assembler'), 'no languages listed', o.frame());
});

await test('Demo: the ASCII table names a clicked glyph; the puzzle slides', async () => {
  const o = await owl();
  const app = new Demo.App(o);
  app.onCommand(Demo.CmAscii);
  const r = o.frame().find('40 ');
  click(o, app, r.x + 3 + 3 + 1, r.y);
  check(o.frame().find('dec 65 hex 41'), 'the click did not name A', o.frame());
  const b = new Board();
  check(b.solved && !b.slide(0) && b.slide(14) && !b.solved && b.slide(15) && b.solved, 'the board rules');
  b.scramble(1);
  check(!b.solved && b.tiles.slice().sort((x, y) => x - y).join() === [...Array(16).keys()].join(), 'a scramble keeps every tile');
});

await test('Demo: Colors recolours the desktop live, and Cancel puts it back', async () => {
  const o = await owl();
  const app = new Demo.App(o);
  const before = o.frame().attr(2, 2);
  app.onCommand(Demo.CmColors);
  app.poll();
  const bg = o.frame().find('Background');
  click(o, app, bg.x + 4, bg.y + 1);
  check((o.frame().attr(2, 2) >> 4) === 1, 'the desktop did not turn blue', o.frame());
  key(o, app, 'Escape');
  check(o.frame().attr(2, 2) === before, 'Cancel did not put the colour back', o.frame());
});

// ------------------------------------------------------------------ files
//
// The server's folder and WebDAV, against RUN.CMD's own server started
// here on a folder of its own; the browser's storage has no Node
// equivalent, so only its message for a browser without one is checked.

const { spawn } = await import('node:child_process');
const { mkdtempSync, writeFileSync, mkdirSync, readFileSync: readText, rmSync, readdirSync } = await import('node:fs');
const { tmpdir } = await import('node:os');
const { join } = await import('node:path');
const { fileURLToPath } = await import('node:url');
const { ServerFolder } = await import('../../../lib/js/files/server.js');
const { WebDavFolder, parseMultistatus } = await import('../../../lib/js/files/webdav.js');
const { BrowserStorage } = await import('../../../lib/js/files/browser.js');

const folder = mkdtempSync(join(tmpdir(), 'owlosui-files-'));
writeFileSync(join(folder, 'WELCOME.TXT'), 'Hello from the server.\n');
mkdirSync(join(folder, 'NOTES'));
writeFileSync(join(folder, 'NOTES', 'A.TXT'), 'note a\n');
const exe = fileURLToPath(new URL(`../../../target/release/owlosui-httpd${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url));
const server = spawn(exe, [fileURLToPath(new URL('../../..', import.meta.url)), '/', '--files', folder, '--no-open']);
const origin = await new Promise((resolve, reject) => {
  let seen = '';
  server.stdout.on('data', d => {
    seen += d;
    const m = seen.match(/at (http:\/\/localhost:\d+)\//);
    if (m) resolve(m[1]);
  });
  server.on('error', reject);
  setTimeout(() => reject(new Error(`no server: ${seen}`)), 10000);
});

for (const [what, source] of [["the server's folder", new ServerFolder(`${origin}/files`)], ['WebDAV', new WebDavFolder(`${origin}/dav/`)]]) {
  await test(`Files: ${what} lists a folder, reads a file and saves one`, async () => {
    const top = await source.list('/');
    check(top.some(e => e.name === 'WELCOME.TXT' && !e.dir && e.size === 23), `no WELCOME.TXT: ${JSON.stringify(top)}`);
    check(top.some(e => e.name === 'NOTES' && e.dir), 'no NOTES folder');
    check((await source.list('/NOTES/')).some(e => e.name === 'A.TXT'), 'no A.TXT in NOTES');
    check((await source.read('/WELCOME.TXT')) === 'Hello from the server.\n', 'read back something else');
    await source.write(`/NOTES/${what.length}.TXT`, 'saved');
    check(readText(join(folder, 'NOTES', `${what.length}.TXT`), 'utf8') === 'saved', 'the save did not reach the disk');
  });
}

await test('Files: a WebDAV answer is read whatever prefix the server gives DAV:', async () => {
  const xml = `<?xml version="1.0"?><d:multistatus xmlns:d="DAV:" xmlns:lp1="DAV:">
    <d:response><d:href>/remote.php/dav/files/me/</d:href><d:propstat><d:prop><lp1:resourcetype><d:collection/></lp1:resourcetype></d:prop></d:propstat></d:response>
    <d:response><d:href>/remote.php/dav/files/me/Photos/</d:href><d:propstat><d:prop><lp1:resourcetype><d:collection/></lp1:resourcetype></d:prop></d:propstat></d:response>
    <d:response><d:href>/remote.php/dav/files/me/Read%20me.md</d:href><d:propstat><d:prop><lp1:resourcetype/><d:getcontentlength>42</d:getcontentlength><d:getlastmodified>Tue, 29 Sep 2026 10:00:00 GMT</d:getlastmodified></d:prop></d:propstat></d:response>
  </d:multistatus>`;
  const e = parseMultistatus(xml, '/remote.php/dav/files/me/');
  check(e.length === 2, `${e.length} entries, not 2: the folder itself is not one of them`);
  check(e[0].name === 'Photos' && e[0].dir, 'Photos is not a folder');
  check(e[1].name === 'Read me.md' && e[1].size === 42 && e[1].date.getUTCDate() === 29, `the file came out ${JSON.stringify(e[1])}`);
});

await test("Files: a browser with no storage for pages is told so in words", async () => {
  let message = '';
  try { await new BrowserStorage().list('/'); } catch (err) { message = err.message; }
  check(/no storage for a page/.test(message), `said: ${message}`);
});

await test("Demo: Open from the server's folder, W and Enter open WELCOME.TXT, F2 saves it", async () => {
  const o = await owl();
  const app = new Demo.App(o);
  app.openFrom(new ServerFolder(`${origin}/files`));
  await app.pending;
  let f = o.frame();
  check(f.find("Open - the server's folder") && f.find('Examples/Web/files on the computer'), 'no Open dialog explaining itself', f);
  check(f.find('WELCOME.TXT') && f.find('NOTES'), 'the folder is not in the panel', f);
  key(o, app, 'w');
  key(o, app, 'Enter');
  await app.pending;
  f = o.frame();
  check(f.find("WELCOME.TXT - the server's folder") && f.find('Hello from the server.'), 'the file did not open', f);
  o.type('Edited. ');
  key(o, app, 'F2');
  await app.pending;
  check(o.frame().find('is saved in'), 'no word that it was saved', o.frame());
  check(readText(join(folder, 'WELCOME.TXT'), 'utf8').startsWith('Edited. Hello'), 'the disk did not change');
});

await test('Demo: a CR LF file opens without a glyph for CR and is saved with CR LF again', async () => {
  writeFileSync(join(folder, 'DOS.TXT'), 'line one\r\nline two\r\n');
  const o = await owl();
  const app = new Demo.App(o);
  app.openFrom(new WebDavFolder(`${origin}/dav/`));
  await app.pending;
  app.chosen('DOS.TXT');
  await app.pending;
  const f = o.frame();
  check(f.find('DOS.TXT - the WebDAV folder') && f.find('line two'), 'the file did not open', f);
  const one = f.find('line one');
  check(one && f.char(one.x + 8, one.y) === ' ', 'a CR shows as a glyph after the line', f);
  o.type('X');
  key(o, app, 'F2');
  await app.pending;
  const disk = readText(join(folder, 'DOS.TXT'), 'utf8');
  check(disk === 'Xline one\r\nline two\r\n', `saved as ${JSON.stringify(disk)}`);
});

await test('Demo: Save and Save as are on File, above Exit, only while a file is in front - F2 too', async () => {
  const o = await owl();
  const app = new Demo.App(o);
  o.press('Escape');
  o.press('f', { alt: true });
  let f = o.frame();
  check(f.find('Exit') && !f.find('Save'), 'Save is offered with no file in front', f);
  check(!f.row(24).includes('F2 Save'), 'F2 Save is on the status line with no file in front', f);
  o.press('Escape');
  o.press('Escape');
  app.onCommand(Demo.CmNew);
  o.press('f', { alt: true });
  f = o.frame();
  const save = f.find('Save  '), as = f.find('Save as'), exit = f.find('Exit');
  check(save && as && exit && save.y < exit.y && as.y < exit.y, 'Save and Save as are not above Exit', f);
  // The open menu's hint has the status line; closed, the keys are back.
  o.press('Escape');
  check(o.frame().row(24).includes('F2 Save'), 'no F2 Save with a file in front', o.frame());
});

await test('Demo: a binary opens in the editor as its bytes and is saved back byte for byte', async () => {
  const all = Uint8Array.from({ length: 256 }, (_, i) => i);
  const original = Buffer.concat([Buffer.from(all), Buffer.from([0, 0, 13, 10, 13, 255, 0]), Buffer.from(all)]);
  writeFileSync(join(folder, 'BYTES.BIN'), original);
  const o = await owl();
  const app = new Demo.App(o);
  app.openFrom(new ServerFolder(`${origin}/files`));
  await app.pending;
  app.chosen('BYTES.BIN');
  await app.pending;
  check(o.frame().find("BYTES.BIN - the server's folder"), 'the binary did not open in an editor', o.frame());
  o.type('X');
  key(o, app, 'F2');
  await app.pending;
  const disk = readFileSync(join(folder, 'BYTES.BIN'));
  const want = Buffer.concat([Buffer.from('X'), original]);
  check(disk.equals(want), `saved ${disk.length} bytes, not the ${want.length} it was plus X; first difference at ${disk.findIndex((b, i) => b !== want[i])}`);
});

await test('Demo: Save as puts the file in front under a new name, asks before replacing, and Save goes there after', async () => {
  rmSync(join(folder, 'COPY.TXT'), { force: true });
  const o = await owl();
  const app = new Demo.App(o);
  app.onCommand(Demo.CmNew);
  const server = new ServerFolder(`${origin}/files`);
  app.saveAs(server);
  await app.pending;
  check(o.frame().find("Save as - the server's folder") && o.frame().find('File name'), 'no Save as dialog', o.frame());
  o.setText(app.open.name, 'COPY.TXT');
  app.onCommand(Demo.CmSaveAsOk);
  await app.pending;
  check(readText(join(folder, 'COPY.TXT'), 'utf8').startsWith('Type here.'), 'COPY.TXT was not written');
  check(o.frame().find("COPY.TXT - the server's folder"), 'the window does not say it is COPY.TXT now', o.frame());
  // Save, from now on, goes to COPY.TXT.
  app.onCommand(Demo.CmDismiss);
  o.type('Again. ');
  key(o, app, 'F2');
  await app.pending;
  check(readText(join(folder, 'COPY.TXT'), 'utf8').startsWith('Again. Type here.'), 'Save did not go to the new name');
  // The same name again: asked first, and Keep it leaves the file alone.
  app.onCommand(Demo.CmDismiss);
  app.saveAs(server);
  await app.pending;
  o.setText(app.open.name, 'copy.txt');
  app.onCommand(Demo.CmSaveAsOk);
  check(o.frame().find('Replace it?'), 'a file was replaced without asking', o.frame());
  app.onCommand(Demo.CmReplace);
  await app.pending;
  check(o.frame().find('is saved in'), 'Replace did not save', o.frame());
});

const { Repository } = await import('../../../lib/js/files/repository.js');

for (const [what, source] of [["the server's folder", new ServerFolder(`${origin}/files`)], ['WebDAV', new WebDavFolder(`${origin}/dav/`)]]) {
  await test(`Files: ${what} makes a folder, renames, writes bytes and deletes`, async () => {
    const tag = what.length;
    await source.mkdir(`/MADE${tag}/`);
    await source.write(`/MADE${tag}/B.BIN`, new Uint8Array([0, 1, 2, 255]));
    check(Buffer.compare(readFileSync(join(folder, `MADE${tag}`, 'B.BIN')), Buffer.from([0, 1, 2, 255])) === 0, 'the bytes changed on the way');
    check((await source.readBytes(`/MADE${tag}/B.BIN`))[3] === 255, 'read back other bytes');
    await source.rename(`/MADE${tag}/B.BIN`, `/MADE${tag}/C.BIN`);
    const names = (await source.list(`/MADE${tag}/`)).map(e => e.name);
    check(names.join() === 'C.BIN', `after the rename: ${names}`);
    await source.remove(`/MADE${tag}/`);
    check(!(await source.list('/')).some(e => e.name === `MADE${tag}`), 'the folder is still there');
  });
}

await test("Files: the examples are listed, read, and refuse to change", async () => {
  const repo = new Repository(`${origin}/Examples`);
  const top = await repo.list('/');
  check(top.some(e => e.name === 'DOS' && e.dir) && top.some(e => e.name === 'README.md' && !e.dir), `listed ${top.map(e => e.name)}`);
  check((await repo.read('/DOS/Pascal/DEMO.PAS')).includes('program'), 'DEMO.PAS did not read');
  let said = '';
  try { await repo.write('/X.TXT', 'x'); } catch (e) { said = e.message; }
  check(/read-only/.test(said), `a write said: ${said}`);
  const r = await fetch(`${origin}/Examples/X.TXT`, { method: 'PUT', body: 'x' });
  check(!r.ok, `the server let a PUT into the repository: ${r.status}`);
});

await test('Commander: two panels, the examples and the server; F5 copies, F6 renames, F7, F8, F3 hex', async () => {
  const o = await owl(100, 30);
  const app = new Demo.App(o);
  const c = app.commander;
  c.sources = [new Repository(`${origin}/Examples`), new ServerFolder(`${origin}/files`), new WebDavFolder(`${origin}/dav/`)];
  app.onCommand(Demo.CmCommander);
  await c.settled;
  let f = o.frame();
  check(f.find('examples:/') && f.find('server:/'), 'the titles do not say where the sides are', f);
  check(f.find('Desktop') && f.find('WELCOME.TXT'), 'the folders are not in the panels', f);
  check(f.find('F5 Copy') && f.find('F8 Delete') && !f.find('F5 Zoom'), "the panel's keys are not on the status line", f);

  // Into DOS/Asm on the left, and HELLO.COM over to the server.
  c.enter(c.left, 'DOS'); await c.settled;
  c.enter(c.left, 'Asm'); await c.settled;
  check(o.frame().find('examples:/DOS/Asm/'), 'the title did not follow', o.frame());
  key(o, app, 'h');
  check(o.markedNames(c.left.files)[0] === 'HELLO.ASM', `the cursor is on ${o.markedNames(c.left.files)}`);
  key(o, app, 'ArrowDown');
  key(o, app, 'F5');
  f = o.frame();
  check(f.find('Copy "HELLO.COM" to') && f.find('server:/'), 'no Copy dialog aimed at the other side', f);
  key(o, app, 'Enter');
  await c.settled;
  const disk = readFileSync(join(folder, 'HELLO.COM'));
  check(Buffer.compare(disk, readFileSync(new URL('../../DOS/Asm/HELLO.COM', import.meta.url))) === 0, 'the copy is not the same bytes');

  // F3 on a program: its bytes.
  key(o, app, 'F3');
  await c.settled;
  check(o.frame().find('HELLO.COM - ') && o.frame().find('00000000'), 'no hex view', o.frame());
  key(o, app, 'F3', { alt: true });
  // Enter on a program: it goes to the DOS PC to run (the PC itself is a
  // browser's; here only the hand-over is seen).
  let ran = null;
  app.dos.runProgram = async (source, dir, name) => { ran = `${source.prefix}${dir}${name}`; };
  c.enter(c.left, 'HELLO.COM');
  await c.settled;
  check(ran === 'examples:/DOS/Asm/HELLO.COM', `Enter on a program ran ${ran}`);

  // Over to the server's side: a folder, the copy renamed, then deleted.
  key(o, app, 'Tab');
  check(o.active() === c.right.win, 'Tab did not go to the other side');
  key(o, app, 'F7');
  o.type('BOX');
  key(o, app, 'Enter');
  await c.settled;
  check(readdirSync(folder).includes('BOX'), 'no BOX folder on disk');
  key(o, app, 'h');
  check(o.markedNames(c.right.files)[0] === 'HELLO.COM', `the cursor is on ${o.markedNames(c.right.files)}`);
  key(o, app, 'F6');
  check(o.frame().find('examples:/DOS/Asm/'), 'the target does not start as the other side', o.frame());
  for (let i = 0; i < 40; i++) key(o, app, 'Backspace');
  o.type('HI.COM');
  key(o, app, 'Enter');
  await c.settled;
  check(readdirSync(folder).includes('HI.COM') && !readdirSync(folder).includes('HELLO.COM'), `renamed to ${readdirSync(folder)}`);
  key(o, app, 'h');
  key(o, app, 'F8');
  check(o.frame().find('Delete "HI.COM"'), 'no question before deleting', o.frame());
  key(o, app, 'Enter');
  await c.settled;
  check(readdirSync(folder).includes('HI.COM'), 'Enter deleted it: No must be the default');
  key(o, app, 'F8');
  check(o.frame().find('Delete "HI.COM"'), 'the second F8 asked nothing', o.frame());
  key(o, app, 'y', { alt: true });
  await c.settled;
  check(!readdirSync(folder).includes('HI.COM'), 'Yes did not delete it');

  // Ctrl+U: the sides change places.
  key(o, app, 'u', { ctrl: true });
  await c.settled;
  check(c.left.source instanceof ServerFolder && c.right.dir === '/DOS/Asm/', 'the sides did not swap');

  // Alt+F3 on a panel closes the commander, both sides.
  key(o, app, 'F3', { alt: true });
  check(!c.left && !o.frame().find('server:/'), 'the commander is still up', o.frame());
  rmSync(join(folder, 'BOX'), { recursive: true, force: true });
});

server.kill();
rmSync(folder, { recursive: true, force: true });

console.log(failed ? `${failed} case(s) FAILED` : 'all cases passed');
process.exit(failed ? 1 : 0);
