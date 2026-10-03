// Web, JavaScript, step 5 of 5 - everything the kit has, in one program, with a menu bar.
// Before: 04-Sokoban. The same demo on the desktop is Examples/Desktop/CSharp/05-OwlosDemo.
//
// The shape every text-mode toolkit's demo has had, because it is the right
// shape: a menu bar, a status line, a desktop, and small windows that each
// use one part of the kit.
//
//   File      New editor windows. Open from: this browser's storage, the
//             server's folder, a WebDAV folder - the file panel, and the
//             file in an editor; Save (F2) puts it back where it came
//             from. lib/js/files/ reads the folders; the panel draws them.
//             Exit.
//   Edit      Only while an editor is in front, and not written here: the
//             editor brings it (owl.editor), and the core answers it.
//   Tools     Commander - two panels over every place above and this
//             project's own examples, Volkov Commander's keys, upload and
//             download (commander.js). Calculator (step 3's, imported as it
//             is), Calendar, ASCII table, Puzzle - tools.js, one class each.
//             Console - everything the page has done since it opened,
//             errors included, in a terminal window, to copy into a bug
//             report (log.js records it, console.js shows it).
//   Options   A dialog of check boxes and radio buttons; the colour dialog,
//             every role of the palette changed live; a ticked item.
//   Window    Size/Move, Zoom, Minimize, Next, Previous, Close, List, Cascade, Tile:
//             the desktop's own verbs. Alt+1..9 reach numbered windows.
//   DOS       A DOS PC in a window: DOSBox in WebAssembly, its disk made
//             of this repository's DOS examples on the same Rust core,
//             floppies that are folders of the browser's storage (DOS A
//             Drive, DOS B Drive), OWL FLY III on the network,
//             settings, a network monitor and a machine monitor (dos.js,
//             monitors.js; the machine is lib/js/dosbox/).
//   Help      What to see - the window a first visit opens on
//             (welcome.js) - and About.
//
// Every tool owns a range of command numbers and answers handles(cmd), so
// this file only routes: a menu command opens a tool, a tool's own buttons
// go to the tool, and poll() lets each look at what is not a command.

import { Style, sub, line, Offer } from '../../../../lib/js/owlosui.js';
import { CalculatorWindow } from '../03-Calculator/calculator.js';
import { CalendarWindow, AsciiTableWindow, PuzzleWindow, ColorsWindow } from './tools.js';
import { BrowserStorage, split } from '../../../../lib/js/files/browser.js';
import { ServerFolder } from '../../../../lib/js/files/server.js';
import { WebDavFolder } from '../../../../lib/js/files/webdav.js';
import { Repository } from '../../../../lib/js/files/repository.js';
import { CommanderTool, binary } from './commander.js';
import { DosTool, DosCm } from './dos.js';
import { Welcome } from './welcome.js';
import { ConsoleWindow } from './console.js';
import { log, watchWindows, mb } from './log.js';

export const CmNew = 1, CmExit = 2;
export const CmCalc = 10, CmCalendar = 11, CmAscii = 12, CmPuzzle = 13, CmCommander = 14;

/**
 * What this browser's storage starts with, the first time it is opened:
 * the demos' own sources for DOS and for Windows, a DOS program and a
 * picture - something real to view, copy, edit and download. Paths from
 * the repository's root, which the page's server serves.
 */
const ROOT = new URL('../../../../', import.meta.url);
const SAMPLES = [
  ['/DOS/DEMO.PAS', 'Examples/DOS/Pascal/DEMO.PAS'],
  ['/DOS/DEMO.C', 'Examples/DOS/C/DEMO.C'],
  ['/DOS/DEMO.ASM', 'Examples/DOS/Asm/DEMO.ASM'],
  ['/DOS/HELLO.COM', 'Examples/DOS/Asm/HELLO.COM'],
  ['/WINDOWS/Program.cs', 'Examples/Desktop/CSharp/01-HelloWorld/Program.cs'],
  ['/WINDOWS/CS_Demo.cmd', 'CS_Demo.cmd'],
  ['/dos_demo.png', 'Examples/screens/dos_demo.png'],
].map(([to, from]) => [to, new URL(from, ROOT).href]);
export const CmOptions = 20, CmClock = 21, CmColors = 22;
export const CmNext = 30, CmZoom = 31, CmClose = 32, CmCascade = 33, CmTile = 34, CmPrevious = 35, CmList = 36, CmSizeMove = 37, CmMinimize = 38;
export const CmAbout = 40, CmHelp = 41, CmWelcome = 42, CmConsole = 43;
export const CmOk = 60, CmCancel = 61, CmDismiss = 62;
export const CmOpenBrowser = 50, CmOpenServer = 51, CmOpenDav = 52, CmSave = 53;
export const CmFileOpen = 54, CmFileCancel = 55, CmDavConnect = 56, CmDavCancel = 57;
export const CmSaveAsBrowser = 64, CmSaveAsServer = 65, CmSaveAsDav = 66, CmSaveAsOk = 67, CmReplace = 68, CmKeep = 69;

export class App {
  constructor(owl) {
    this.owl = owl;
    // Every window from here on is in the console's record, opened and closed.
    this.windows = watchWindows(owl);
    log.info('page', `the core is running: ${owl.width}x${owl.height} cells`);
    this.calc = new CalculatorWindow(owl, CmClose);
    this.calendar = new CalendarWindow(owl, CmClose);
    this.ascii = new AsciiTableWindow(owl, CmClose);
    this.puzzle = new PuzzleWindow(owl, CmClose);
    this.colors = new ColorsWindow(owl);
    this.tools = [this.calc, this.calendar, this.ascii, this.puzzle];
    this.editors = [];
    this.docs = new Map();            // window -> { text, source, path, name }
    this.open = null;                 // the Open dialog, while it is up
    this.dav = null;                  // the WebDAV address dialog, likewise
    this.browser = new BrowserStorage({ samples: SAMPLES });
    // The commander's drives: every place a page can keep files, and the
    // examples themselves to copy from.
    const origin = globalThis.location?.origin ?? 'http://localhost:8765';
    this.commander = new CommanderTool(owl, {
      sources: [new Repository(), this.browser, new ServerFolder(), new WebDavFolder(`${origin}/dav/`)],
      // A file opened from the commander fills the desktop, as a
      // commander's editor always has; F5 puts it back to a window.
      open: (name, text, source, path, options) => owl.zoom(this.addDoc(name, text, source, path, options)),
      closeCmd: CmClose,
      quietKeys: [{ cmd: CmHelp, key: 'F1' }, { cmd: CmClose, key: 'F3', alt: true }, { cmd: CmExit, key: 'x', alt: true }],
      // Enter on a DOS program: it runs on the DOS PC, in its window.
      run: (source, dir, name) => this.dos.runProgram(source, dir, name),
    });
    this.tools.push(this.commander);
    // The DOS PC. Its floppies are folders of this browser's storage,
    // DOS A Drive and DOS B Drive: the commander reaches them there.
    this.dos = new DosTool(owl, {
      commander: this.commander,
      storage: this.browser,
      open: (name, text, source, path, options) => this.addDoc(name, text, source, path, options),
    });
    this.tools.push(this.dos);
    this.welcome = new Welcome(owl, this.choices());
    this.tools.push(this.welcome);
    this.console = new ConsoleWindow(owl, { state: () => this.state() });
    this.tools.push(this.console);
    this.dropServerIfAbsent();
    this.options = 0;
    this.box = 0;
    this.clock = true;

    // A hint is one line the status line shows while the cursor stands on
    // the item, in place of the keys.
    owl.menuBar(
      sub('~F~ile',
        { label: '~N~ew', cmd: CmNew, shortcut: 'F3', hint: 'An editor window of its own' },
        sub('~O~pen from',
          { label: "~T~his browser's storage...", cmd: CmOpenBrowser,
            hint: 'Files this browser keeps for this page, on this computer; nothing is uploaded' },
          { label: "The ~s~erver's folder...", cmd: CmOpenServer,
            hint: 'Examples/Web/files on the computer running RUN.CMD' },
          { label: 'A ~W~ebDAV folder...', cmd: CmOpenDav,
            hint: 'A folder shared the way a NAS or Nextcloud shares one; RUN.CMD shares one too' }),
        // Save and Save as are not here: an editor window brings them, above
        // Exit, while it is in front (addDoc). With no file in front there
        // is nothing to save, and the menu does not offer it.
        line(),
        { label: 'E~x~it', cmd: CmExit, shortcut: 'Alt+X', hint: 'Leave the program' }),
      sub('~T~ools',
        { label: 'Co~m~mander', cmd: CmCommander, hint: 'Two panels of files: the examples, this browser, the server, WebDAV; F2 uploads and downloads' },
        line(),
        { label: '~C~alculator', cmd: CmCalc, hint: 'Arithmetic, trigonometry, hex and binary' },
        { label: 'Ca~l~endar', cmd: CmCalendar, hint: 'A month at a time; click a day' },
        { label: '~A~SCII table', cmd: CmAscii, hint: 'Every glyph of the font, by its number' },
        { label: '~P~uzzle', cmd: CmPuzzle, hint: 'The fifteen puzzle' },
        line(),
        { label: 'C~o~nsole', cmd: CmConsole, hint: 'Everything the page and the DOS PC have done since it opened, errors too: copy it into a bug report' }),
      sub('~D~OS', ...this.dos.menu()),
      sub('~O~ptions',
        { label: '~M~ouse...', cmd: CmOptions, hint: 'Check boxes and radio buttons, read back on OK' },
        { label: 'Co~l~ors...', cmd: CmColors, hint: "Every role's colour, changed live" },
        { label: '~C~lock on status line', cmd: CmClock, checked: this.clock, hint: 'A ticked option: on or off' }),
      sub('~W~indow',
        { label: '~S~ize/Move', cmd: CmSizeMove, shortcut: 'Ctrl+F5', hint: 'Arrows move the window, Shift+arrows resize it; Enter keeps, Esc puts back' },
        { label: '~Z~oom', cmd: CmZoom, shortcut: 'F5', hint: 'The window fills the desktop, or goes back to its size' },
        { label: 'Mi~n~imize', cmd: CmMinimize, hint: 'The window falls into a bar in the bottom right corner; a click on the bar brings it back' },
        { label: '~N~ext', cmd: CmNext, shortcut: 'F6', hint: 'The front window goes to the back' },
        { label: '~P~revious', cmd: CmPrevious, shortcut: 'Shift+F6', hint: 'The window at the back comes to the front' },
        { label: '~C~lose', cmd: CmClose, shortcut: 'Alt+F3', hint: 'Close the front window' },
        { label: '~L~ist...', cmd: CmList, shortcut: 'Alt+0', hint: 'Every window by number; Enter brings one to the front' },
        line(),
        { label: 'C~a~scade', cmd: CmCascade, hint: 'The windows along the diagonal, every title showing' },
        { label: '~T~ile', cmd: CmTile, hint: 'The windows share the desktop with no overlap' }),
      sub('~H~elp',
        { label: '~W~hat to see...', cmd: CmWelcome, hint: 'The window this page opened with: what there is, and Enter to see it' },
        { label: '~A~bout', cmd: CmAbout, hint: 'What this program is and what draws it' }),
    );
    owl.statusLine(
      { label: '~F1~ Help', cmd: CmHelp, key: 'F1' },
      { label: '~F3~ New', cmd: CmNew, key: 'F3' },
      { label: '~F5~ Zoom', cmd: CmZoom, key: 'F5' },
      { label: '~F6~ Next', cmd: CmNext, key: 'F6' },
      { label: '~Alt-F3~ Close', cmd: CmClose, key: 'F3', alt: true },
      { label: '~Alt-X~ Exit', cmd: CmExit, key: 'x', alt: true },
    );
  }

  /** What the What to see window offers, and what each one does. */
  choices() {
    const show = (...cmds) => () => { for (const c of cmds) this.onCommand(c); };
    return [
      { label: 'Files, in this browser', about: "Tools > Commander: two panels, Volkov Commander's keys. This project's examples on one side, " +
        "this browser's own storage on the other. F3 views, F4 edits, F5 copies, F2 uploads from your computer and downloads to it.",
        go: () => this.commander.show({ left: this.commander.sources[0], right: this.browser }) },
      { label: 'OWLOSUI on DOS, in DOSBox', about: 'A DOS PC in a window - DOSBox, compiled to WebAssembly - running the same commander, ' +
        'written in Pascal, on the same Rust core, built for DOS. Click the picture for the keyboard; Right Ctrl gives it back.',
        go: () => this.dos.run('commander') },
      { label: 'Your files in DOS', about: "In this browser's storage, the folder DOS A Drive is drive A: of the DOS PC, and DOS B Drive " +
        'is B:. F5 copies a file into it, F2 uploads one from your computer: DOS sees it at once, and Enter there runs it. ' +
        'What DOS saves on A:, the folder keeps.',
        go: () => this.floppyDemo() },
      { label: 'Play OWL FLY III', about: 'A DOS flight game over the network: each IPX packet goes in a WebSocket to a relay, to every ' +
        'player in the room. DOS > Network monitor shows the traffic, packet by packet.',
        go: () => this.dos.run('owlfly') },
      { label: 'Calculator, calendar, puzzle', about: 'Small windows that each use one part of the kit: button rows, a canvas of cells, ' +
        'check boxes. All of them are under Tools, and Window arranges them.',
        go: show(CmCalc, CmCalendar, CmPuzzle) },
      { label: 'DOSBox settings', about: 'Video card, CPU, sound, memory, network: a page each, written out as the dosbox.conf the ' +
        'DOS PC starts with - the dialog shows it.',
        go: show(DosCm.Settings) },
      { label: 'What the page is doing', about: 'Tools > Console: everything since the page opened - windows, the DOS PC switching ' +
        'on, what DOSBox says, the network, every error - in a terminal window, coloured by ANSI sequences as a terminal is. ' +
        'F4 adds the state of everything now; Ctrl+C copies it all for a bug report.',
        go: show(CmConsole) },
      { label: 'Look around myself', about: 'F10 opens the menu bar; every item says on the status line what it does. ' +
        'Help > What to see brings this window back.',
        go: () => {} },
    ];
  }

  /**
   * The browser's files on the left; on the right, DOS A Drive - with the
   * DOS PC over it, A: open. F5 on the left copies into the folder on the
   * right, and so onto the floppy in front of you.
   */
  floppyDemo() {
    this.commander.show({ left: this.browser, right: this.browser, rightDir: '/DOS A Drive/' });
    const r = this.commander.rightRect();
    this.dos.run('commander', { rect: r, grab: false }).then(() => {
      if (this.commander.left) this.owl.activate(this.commander.left.win);
      this.owl.refresh?.();
    });
  }

  /**
   * What everything is doing now, a line each, for Console > State now
   * (F4): the page, its windows, the DOS PC, the network.
   */
  async state() {
    const owl = this.owl, dos = this.dos, box = dos.box;
    const lines = [];
    const m = globalThis.performance?.memory;
    lines.push(`page: window ${innerWidth}x${innerHeight}, ${owl.width}x${owl.height} cells, device pixels ${devicePixelRatio}, ` +
      `${document.hidden ? 'hidden' : 'visible'}, ${document.hasFocus() ? 'focused' : 'not focused'}` +
      (m ? `, memory ${mb(m.usedJSHeapSize)} of ${mb(m.jsHeapSizeLimit)}` : ''));
    const active = owl.active();
    const open = [...this.windows].map(([id, title]) => `#${id} "${title}"${id === active ? ' (front)' : ''}`);
    lines.push(`windows: ${open.length ? open.join(', ') : 'none'}`);
    const est = await navigator.storage?.estimate?.().catch(() => null);
    if (est) lines.push(`storage: ${mb(est.usage)} used of ${mb(est.quota)}`);
    const s = dos.settings;
    lines.push(`dos settings: ${s.machine}, ${s.memsize} MB, cpu ${s.core}/${s.cputype}, cycles ${s.cycles === 'fixed' ? `fixed ${s.fixed}` : s.cycles}, ` +
      `${s.sbtype}, ems ${s.ems ? 'on' : 'off'}, umb ${s.umb ? 'on' : 'off'}, awake ${s.awake ? 'on' : 'off'}, picture ${s.picture}, ` +
      `ipx ${s.ipx ? 'on' : 'off'}, relay ${s.relay}, room ${s.room}`);
    if (!box.running) {
      lines.push('dos: off');
    } else {
      lines.push(`dos: on, running ${dos.program?.name ?? dos.started}, keyboard ${box.hasKeys() ? 'in DOS' : 'on the page'}, ` +
        `picture ${box.frameSize ? `${box.frameSize.w}x${box.frameSize.h}` : 'not yet'}, ${box.keepRunning ? 'awake' : 'pauses when hidden'}`);
      const st = await box.stats().catch(() => null);
      if (st) {
        const { net, ...rest } = st;
        lines.push(`dos emulator: ${Object.entries(rest).map(([k, v]) => `${k} ${v}`).join(', ')}`);
        lines.push(`net: ${dos.network.state}${dos.network.url ? ` ${dos.network.url}` : ''}; ` +
          `sent ${net.packetsOut} packets (${net.sent} bytes), received ${net.packetsIn} (${net.received} bytes)`);
      }
    }
    return lines;
  }

  /**
   * The server's folder and WebDAV are RUN.CMD's; a page served from
   * anywhere else - GitHub Pages - has neither, and the commander does
   * not offer drives that cannot answer.
   */
  async dropServerIfAbsent() {
    if (typeof fetch === 'undefined' || typeof location === 'undefined') return;
    const ok = await fetch(`${location.origin}/files/?list`).then(r => r.ok, () => false);
    if (!ok) this.commander.sources = this.commander.sources.filter(s => !(s instanceof ServerFolder || s instanceof WebDavFolder));
  }

  tell(title, text) {
    this.closeBox();
    this.box = this.owl.messageBox(title, text, ['~O~K', CmDismiss]);
  }

  closeBox() {
    if (this.box) this.owl.close(this.box);
    this.box = 0;
  }

  /** File > New: an editor of its own, a window like any other. */
  newEditor() {
    const n = this.editors.length + 1;
    this.addDoc(`UNTITLED${n}.TXT`, 'Type here. Ctrl+Z takes it back, Shift and the arrows select.\n' +
      "F2 saves it in this browser's storage.\n", null, null);
  }

  /**
   * An editor window for a file; `source` and `path` say where Save puts it
   * back. `text` is a string, or the file's bytes: a program, a picture,
   * anything that is not a text opens as its bytes, one glyph each, as a
   * DOS editor opened one - garbage on the screen, Edit > Hex view to see
   * the numbers - and is saved back byte for byte.
   */
  addDoc(name, text, source, path, { readOnly = false } = {}) {
    const owl = this.owl;
    const n = this.editors.length + 1;
    const w = owl.window(this.docTitle(name, source), 60, 16, { x: 2 + (n % 8), y: 1 + (n % 8), closeCmd: CmClose });
    const binary = text instanceof Uint8Array;
    // A DOS or Windows file ends its lines with CR LF. The editor wants LF
    // alone - a CR would show as a glyph - so the CRs come off here and go
    // back on in save(), and the file keeps the line ends it came with.
    // A file opened as bytes keeps its CRs: they are bytes like the others.
    const crlf = !binary && text.includes('\r\n');
    const t = owl.text(w, binary ? owl.textOfBytes(text) : crlf ? text.replace(/\r\n/g, '\n') : text);
    // Everything the editor has, on an Edit menu of its own while this
    // window is in front: Find, Replace, Word wrap, Read only, Hex view,
    // Classic keys. The core runs all of it; nothing comes back here.
    owl.editor(t, Offer.All, { readOnly });
    // Coloured as its language, which its name says: DEMO.PAS is Pascal.
    // A name no language answers to stays plain, and Edit > Syntax can
    // still choose one.
    if (!binary) owl.syntax(t, name);
    // Save and Save as, on the File menu above Exit and F2 on the status
    // line, while this window is in front - and only then.
    owl.windowMenu(w, sub('~F~ile',
      { label: '~S~ave', cmd: CmSave, shortcut: 'F2', hint: 'This file, back where it came from' },
      sub('Save ~a~s',
        { label: "~T~his browser's storage...", cmd: CmSaveAsBrowser, hint: 'Under a name and in a folder you choose, in this browser' },
        { label: "The ~s~erver's folder...", cmd: CmSaveAsServer, hint: 'Into Examples/Web/files on the computer running RUN.CMD' },
        { label: 'A ~W~ebDAV folder...', cmd: CmSaveAsDav, hint: 'Into a folder shared the way a NAS or Nextcloud shares one' })));
    owl.windowStatus(w, { label: '~F2~ Save', cmd: CmSave, key: 'F2' });
    this.editors.push(w);
    this.docs.set(w, { text: t, source, path, name, crlf, binary });
    return w;
  }

  docTitle(name, source) { return source ? `${name} - ${source.title}` : name; }

  // ---------------------------------------------------------------- files
  //
  // Every source answers list(dir), read(path) and write(path, text), and
  // says in `about` what it is in words a first-time visitor knows. The
  // panel only draws: the program reads the folder and hands it over.

  /** Work that finishes later: what goes wrong is said in a box, and a test can wait for it. */
  track(promise) {
    this.pending = promise.catch(e => this.tell('Files', e.message)).finally(() => this.owl.refresh?.());
    return this.pending;
  }

  /**
   * File > Open from: the Open dialog, the source explained above the
   * panel. With `saveAs`, File > Save as: the same dialog, walking the
   * folders, with a line for the name the file in front is saved under.
   */
  openFrom(source, saveAs = null) {
    const owl = this.owl;
    this.closeOpen();
    const d = owl.window(`${saveAs ? 'Save as' : 'Open'} - ${source.title}`, 72, saveAs ? 23 : 21,
      { style: Style.ModalDialog, closeCmd: CmFileCancel });
    // The panel first: the core gives a window's keys to a file panel only
    // when it is the window's first part. The words about the source go in
    // the rows it leaves free above itself.
    const panel = owl.files(d, `${source.prefix}/*.*`, [], { top: saveAs ? 5 : 3 });
    owl.staticText(d, 2, 1, source.about, 66, 2);
    let name = 0;
    if (saveAs) {
      // Between the words and the panel: a name chosen in the panel goes
      // here, and Save writes into the folder the panel shows.
      name = owl.input(d, 2, 3, 50, 'File name', saveAs.name);
      owl.buttons(d, { label: '~S~ave', cmd: CmSaveAsOk, default: true }, { label: '~C~ancel', cmd: CmFileCancel, cancel: true });
    } else {
      owl.buttons(d, { label: '~O~pen', cmd: CmFileOpen, default: true }, { label: '~C~ancel', cmd: CmFileCancel, cancel: true });
    }
    this.open = { dialog: d, panel, source, dir: '/', entries: [], saveAs, name };
    return this.track(this.showFolder('/'));
  }

  closeOpen() {
    if (this.open) this.owl.close(this.open.dialog);
    this.open = null;
  }

  /** A folder of the source into the panel - or, in the panel, why not. */
  async showFolder(dir) {
    const o = this.open;
    if (!o) return;
    try {
      const entries = await o.source.list(dir);
      if (dir !== '/') entries.unshift({ name: '..', size: 0, date: new Date(), dir: true });
      if (this.open !== o) return; // closed while the folder was on its way
      o.dir = dir;
      o.entries = entries;
      this.owl.setFiles(o.panel, `${o.source.prefix}${dir}*.*`, entries);
    } catch (e) {
      if (this.open === o) this.owl.filesError(o.panel, e.message);
    }
  }

  /**
   * A file into an editor window; the dialog closes. Read as bytes: a text
   * opens as text, anything else as its bytes (addDoc). In Save as, a file
   * chosen in the panel is the name to save under instead.
   */
  async openFile(path) {
    const o = this.open;
    if (!o) return;
    if (o.saveAs) {
      this.owl.setText(o.name, split(path).name);
      return this.saveAsHere();
    }
    try {
      const bytes = await o.source.readBytes(path);
      this.closeOpen();
      this.addDoc(split(path).name, binary(bytes) ? bytes : new TextDecoder().decode(bytes), o.source, path);
    } catch (e) {
      if (this.open === o) this.owl.filesError(o.panel, e.message);
    }
  }

  /**
   * A name entered in the panel: a folder is walked into, a file opened.
   * Enter on a name is reported twice - the panel says which name, and the
   * dialog's default button, Open, is pressed - and reading takes a while
   * here, so while one is on its way the second is let go.
   */
  chosen(name) {
    const o = this.open;
    if (!o || o.busy) return;
    o.busy = true;
    const e = o.entries.find(x => x.name === name);
    const work = name === '..' ? this.showFolder(o.dir.replace(/[^/]+\/$/, ''))
      : e?.dir ? this.showFolder(`${o.dir}${name}/`)
        : this.openFile(`${o.dir}${name}`);
    return this.track(work.finally(() => { o.busy = false; }));
  }

  /** A path typed in the panel's path line: a folder, a mask, or a file. */
  typed(text) {
    const o = this.open;
    let p = text.startsWith(o.source.prefix) ? text.slice(o.source.prefix.length) : text;
    if (!p.startsWith('/')) p = o.dir + p;
    if (/[*?]/.test(p)) p = p.slice(0, p.lastIndexOf('/') + 1);
    return p.endsWith('/') ? this.track(this.showFolder(p)) : this.track(this.openFile(p));
  }

  /** What a document holds, as the file will have it: its text with its own line ends, or its bytes. */
  contentOf(doc) {
    if (doc.binary) return this.owl.getTextBytes(doc.text);
    const typed = this.owl.getText(doc.text);
    return doc.crlf ? typed.replace(/\r?\n/g, '\r\n') : typed;
  }

  /**
   * File > Save (F2): back where it came from; a new file goes to this
   * browser's storage. Only an editor's window offers it, so there is
   * always a file in front - the check is for a program that calls this.
   */
  save() {
    const w = this.owl.active();
    const doc = this.docs.get(w);
    if (!doc) return null;
    if (!doc.source) {
      doc.source = this.browser;
      doc.path = `/${doc.name}`;
      this.owl.setText(w, this.docTitle(doc.name, doc.source));
    }
    const content = this.contentOf(doc);
    return this.track((async () => {
      await doc.source.write(doc.path, content);
      this.tell('Saved', `${doc.name} is saved in ${doc.source.title}, as ${doc.path}. File > Open from finds it there.`);
    })());
  }

  /** File > Save as: the dialog over a source, for the file in front. */
  saveAs(source) {
    const w = this.owl.active();
    const doc = this.docs.get(w);
    if (!doc) return null;
    return this.openFrom(source, { window: w, name: doc.name });
  }

  /**
   * Save in the Save as dialog: the name typed, in the folder the panel
   * shows. A name already there is asked about first; a folder's name is
   * walked into, as Enter on it would.
   */
  saveAsHere(replace = false) {
    const o = this.open;
    if (!o?.saveAs || o.saving || (o.asking && !replace)) return null;
    o.asking = false;
    const name = this.owl.getText(o.name).trim();
    if (!name || /[\\:*?"<>|]/.test(name)) {
      this.owl.filesError(o.panel, name ? `"${name}" cannot be a file's name.` : 'Type a name to save under.');
      return null;
    }
    if (name.includes('/')) return this.typed(name);
    const there = o.entries.find(e => e.name.toLowerCase() === name.toLowerCase());
    if (there?.dir) return this.track(this.showFolder(`${o.dir}${there.name}/`));
    if (there && !replace) {
      o.asking = true;
      this.box = this.owl.messageBox('Save as', `${there.name} is already in ${o.source.title}${o.dir}. Replace it?`,
        { label: '~R~eplace', cmd: CmReplace }, { label: '~K~eep it', cmd: CmKeep, default: true, cancel: true });
      return null;
    }
    const doc = this.docs.get(o.saveAs.window);
    if (!doc) { this.closeOpen(); return null; }
    const path = `${o.dir}${there?.name ?? name}`;
    const content = this.contentOf(doc);
    o.saving = true;
    return this.track((async () => {
      try {
        await o.source.write(path, content);
      } catch (e) {
        o.saving = false;
        if (this.open === o) this.owl.filesError(o.panel, e.message);
        return;
      }
      this.closeOpen();
      // From now on the window is that file: Save goes there, and the
      // title says so.
      Object.assign(doc, { source: o.source, path, name: split(path).name });
      this.owl.setText(o.saveAs.window, this.docTitle(doc.name, doc.source));
      if (!doc.binary) this.owl.syntax(doc.text, doc.name);
      this.tell('Saved', `${doc.name} is saved in ${doc.source.title}, as ${path}.`);
    })());
  }

  /** File > Open from > A WebDAV folder: where the share is, first. */
  davAsk(forSave = false) {
    const owl = this.owl;
    this.davForSave = forSave;
    if (this.dav) { owl.activate(this.dav.dialog); return; }
    const d = owl.window('A WebDAV folder', 68, 16, { style: Style.ModalDialog, closeCmd: CmDavCancel });
    owl.staticText(d, 2, 1, 'WebDAV is how NAS boxes, Nextcloud and many servers share folders. The address ' +
      "below is RUN.CMD's own share of Examples/Web/files, the same files as the server's folder: leave it " +
      'to try, or type another.', 62, 4);
    const origin = globalThis.location?.origin ?? 'http://localhost:8765';
    // An input's label is plain words; Tab walks the fields.
    const url = owl.input(d, 2, 6, 62, 'Address', `${origin}/dav/`);
    const user = owl.input(d, 2, 8, 40, 'User, if it asks', '');
    const password = owl.input(d, 2, 10, 40, 'Password, shown as typed', '');
    owl.buttons(d, { label: 'C~o~nnect', cmd: CmDavConnect, default: true }, { label: '~C~ancel', cmd: CmDavCancel, cancel: true });
    this.dav = { dialog: d, url, user, password };
  }

  closeDav() {
    if (this.dav) this.owl.close(this.dav.dialog);
    this.dav = null;
  }

  davConnect() {
    const v = id => this.owl.getText(id).trim();
    const source = new WebDavFolder(v(this.dav.url), { user: v(this.dav.user), password: v(this.dav.password) });
    const forSave = this.davForSave;
    this.closeDav();
    return forSave ? this.saveAs(source) : this.openFrom(source);
  }

  showOptions() {
    const owl = this.owl;
    if (this.options) { owl.activate(this.options); return; }
    this.options = owl.window('Mouse', 40, 13, { style: Style.ModalDialog, closeCmd: CmCancel });
    owl.label(this.options, 1, 1, '~B~uttons:');
    this.optChecks = owl.cluster(this.options, 2, 2, 30, ['~R~everse buttons', '~S~how cursor']);
    owl.label(this.options, 1, 5, '~D~ouble-click speed:');
    this.optRadio = owl.cluster(this.options, 2, 6, 30, ['S~l~ow', '~M~edium', '~F~ast'], { single: true });
    owl.buttons(this.options, { label: '~O~K', cmd: CmOk, default: true }, ['~C~ancel', CmCancel]);
  }

  /** The front window is being closed: whoever owns it forgets it. */
  closeActive() {
    const owl = this.owl, a = owl.active();
    if (!a) return;
    for (const t of [this.commander, this.dos, this.welcome, this.console]) if (t.owns(a)) { t.closeWindow(a); return; }
    const tool = this.tools.find(t => t.id === a);
    owl.close(a);
    if (tool) tool.closed();
    this.editors = this.editors.filter(e => e !== a);
    this.docs.delete(a);
    if (a === this.options) this.options = 0;
    if (this.open?.dialog === a) this.open = null;
    if (this.dav?.dialog === a) this.dav = null;
  }

  onCommand(cmd) {
    const owl = this.owl;
    switch (cmd) {
      case CmExit: return false;
      case CmHelp:
      case CmWelcome: this.welcome.show(); return true;
      case CmConsole: this.console.show(); return true;
      case CmAbout:
        this.tell('OWLOS UI Demo', 'A text mode toolkit in the classic DOS style, with one portable core: this page is JavaScript, ' +
          'every window in it is drawn by a Rust core compiled to WebAssembly, and the same core runs on a ' +
          'terminal, in a Windows window and on DOS.');
        return true;
      case CmDismiss: this.closeBox(); return true;
      case CmNew: this.newEditor(); return true;
      case CmOpenBrowser: this.openFrom(this.browser); return true;
      case CmOpenServer: this.openFrom(new ServerFolder()); return true;
      case CmOpenDav: this.davAsk(); return true;
      case CmDavConnect: this.davConnect(); return true;
      case CmDavCancel: this.closeDav(); return true;
      case CmSave: this.save(); return true;
      case CmSaveAsBrowser: this.saveAs(this.browser); return true;
      case CmSaveAsServer: this.saveAs(new ServerFolder()); return true;
      case CmSaveAsDav: this.davAsk(true); return true;
      case CmSaveAsOk: {
        // Enter on a name in the panel presses Save too, and before the
        // panel's own report is collected: take that first, so the name
        // under the cursor is the one saved to, not the one in the line.
        const o = this.open;
        const { kind, text } = o ? owl.takeFiles(o.panel) : { kind: 0 };
        if (kind === 1) this.chosen(text);
        else if (kind === 2) this.typed(text);
        else this.saveAsHere();
        return true;
      }
      case CmReplace: this.closeBox(); this.saveAsHere(true); return true;
      case CmKeep: this.closeBox(); if (this.open) this.open.asking = false; return true;
      case CmFileCancel: this.closeOpen(); return true;
      case CmFileOpen: {
        // The Open button: whatever is under the cursor.
        const [name] = this.open ? owl.markedNames(this.open.panel) : [];
        if (name) this.chosen(name);
        return true;
      }
      case CmCommander: this.commander.show(); return true;
      case CmCalc: this.calc.show(); return true;
      case CmCalendar: this.calendar.show(); return true;
      case CmAscii: this.ascii.show(); return true;
      case CmPuzzle: this.puzzle.show(); return true;
      case CmOptions: this.showOptions(); return true;
      case CmColors: this.colors.show(); return true;
      case CmClock:
        this.clock = !this.clock;
        owl.menuCheck(CmClock, this.clock);
        return true;
      case CmOk: {
        const { on } = owl.clusterState(this.optChecks);
        const speed = owl.clusterState(this.optRadio).on.indexOf(true);
        owl.close(this.options);
        this.options = 0;
        this.tell('Options', `Reverse buttons: ${on[0] ? 'on' : 'off'}. Show cursor: ${on[1] ? 'on' : 'off'}. ` +
          `Speed: ${['slow', 'medium', 'fast'][Math.max(0, speed)]}.`);
        return true;
      }
      case CmCancel:
        if (this.options) { owl.close(this.options); this.options = 0; }
        return true;
      case CmNext: owl.nextWindow(); return true;
      case CmPrevious: owl.previousWindow(); return true;
      case CmZoom: { const a = owl.active(); if (a) owl.zoom(a); return true; }
      case CmMinimize: { const a = owl.active(); if (a) owl.minimize(a); return true; }
      case CmClose: this.closeActive(); return true;
      case CmCascade: owl.cascade(); return true;
      case CmTile: owl.tile(); return true;
      case CmList: owl.windowList(); return true;
      case CmSizeMove: owl.sizeMove(); return true;
      default:
        // A tool's own buttons go to the tool.
        for (const t of [this.colors, ...this.tools]) if (t.handles(cmd)) { t.onCommand(cmd); break; }
        return true;
    }
  }

  poll() {
    for (const t of [...this.tools, this.colors]) t.poll();
    // What the file panel reports is not a command.
    if (this.open) {
      const { kind, text } = this.owl.takeFiles(this.open.panel);
      if (kind === 1) this.chosen(text);
      else if (kind === 2) this.typed(text);
    }
  }
}
