// Web, JavaScript, step 5 of 5 - everything the kit has, in one program, with a menu bar.
// Before: 04-Sokoban. The same demo on the desktop is Examples/Desktop/CSharp/05-OwlosDemo.
//
// The shape every text-mode toolkit's demo has had, because it is the right
// shape: a menu bar, a status line, a desktop, and small windows that each
// use one part of the kit.
//
//   File      New editor windows. Open from: this browser's storage, the
//             server's folder, a WebDAV folder - the file panel, and the
//             file in an editor; while one is in front, Save (F2) puts it
//             back where it came from and Save as somewhere else.
//             lib/js/files/ reads the folders, lib/js/apps/documents.js is
//             the editors and the dialog. Exit.
//   Edit      Only while an editor is in front, and not written here: the
//             editor brings it (owl.editor), and the core answers it.
//   Tools     Commander - two panels over every place above and this
//             project's own examples, Volkov Commander's keys, upload and
//             download (lib/js/apps/commander.js). Calculator (step 3's,
//             imported as it is), Calendar, ASCII table, Puzzle - tools.js,
//             one class each. Console - everything the page has done since
//             it opened, errors included, in a terminal window, to copy
//             into a bug report (lib/js/apps/log.js records it, console.js
//             there shows it).
//   Options   A dialog of check boxes and radio buttons; the colour dialog,
//             every role of the palette changed live; a ticked item.
//   Window    Size/Move, Zoom, Minimize, Next, Previous, Close, List, Cascade, Tile:
//             the desktop's own verbs. Alt+1..9 reach numbered windows.
//   DOS       A DOS PC in a window: DOSBox in WebAssembly, its disk the
//             library's toolkit and this repository's DOS examples - the
//             commander among them - on the same Rust core, floppies that are folders of
//             the browser's storage (DOS A Drive, DOS B Drive), OWL FLY III
//             on the network, settings, a network monitor and a machine
//             monitor (lib/js/apps/dos.js and monitors.js; the machine is
//             lib/js/dosbox/). What is the demo's own - its starts, its
//             files on C:, its batch files - is DOS_STARTS, DOS_DISK and
//             DOS_WRITTEN below.
//   Help      What to see - the window a first visit opens on
//             (welcome.js) - and About.
//
// Every tool owns a range of command numbers and answers handles(cmd), so
// this file only routes: a menu command opens a tool, a tool's own buttons
// go to the tool, and poll() lets each look at what is not a command.

import { Style, sub, line } from '../../../../lib/js/owlosui.js';
import { CalculatorWindow } from '../03-Calculator/calculator.js';
import { CalendarWindow, AsciiTableWindow, PuzzleWindow, ColorsWindow } from './tools.js';
import { BrowserStorage } from '../../../../lib/js/files/browser.js';
import { ServerFolder } from '../../../../lib/js/files/server.js';
import { WebDavFolder } from '../../../../lib/js/files/webdav.js';
import { Repository } from '../../../../lib/js/files/repository.js';
import { CommanderTool } from '../../../../lib/js/apps/commander.js';
import { Documents, DocCm } from '../../../../lib/js/apps/documents.js';
import { DosTool, DosCm } from '../../../../lib/js/apps/dos.js';
import { ConsoleWindow } from '../../../../lib/js/apps/console.js';
import { log, watchWindows, mb } from '../../../../lib/js/apps/log.js';
import { Welcome } from './welcome.js';

export { DocCm, DosCm };
export const CmExit = 2;
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

/**
 * The DOS PC's disk C:, beyond the toolkit the library puts there: this
 * repository's DOS examples - the commander, the demo in three languages -
 * with their sources [where on C:, where in the repository]; and OWL FLY
 * III, as a bundle.
 */
const DOS_DISK = [
  ['DEMO/COMMANDR/COMMANDR.EXE', 'Examples/DOS/Commandr/COMMANDR.EXE'],
  ['DEMO/COMMANDR/COMMANDR.PAS', 'Examples/DOS/Commandr/COMMANDR.PAS'],
  ['DEMO/PASCAL/DEMO.EXE', 'Examples/DOS/Pascal/DEMO.EXE'],
  ['DEMO/PASCAL/DEMO.PAS', 'Examples/DOS/Pascal/DEMO.PAS'],
  ['DEMO/PASCAL/HELLO.EXE', 'Examples/DOS/Pascal/HELLO.EXE'],
  ['DEMO/PASCAL/HELLO.PAS', 'Examples/DOS/Pascal/HELLO.PAS'],
  ['DEMO/PASCAL/OWLOSUI.PAS', 'lib/dos/pascal/OWLOSUI.PAS'],
  ['DEMO/C/DEMO.EXE', 'Examples/DOS/C/DEMO.EXE'],
  ['DEMO/C/DEMO.C', 'Examples/DOS/C/DEMO.C'],
  ['DEMO/C/HELLO.EXE', 'Examples/DOS/C/HELLO.EXE'],
  ['DEMO/C/HELLO.C', 'Examples/DOS/C/HELLO.C'],
  ['DEMO/ASM/DEMO.COM', 'Examples/DOS/Asm/DEMO.COM'],
  ['DEMO/ASM/DEMO.ASM', 'Examples/DOS/Asm/DEMO.ASM'],
  ['DEMO/ASM/HELLO.COM', 'Examples/DOS/Asm/HELLO.COM'],
  ['DEMO/ASM/HELLO.ASM', 'Examples/DOS/Asm/HELLO.ASM'],
].map(([to, from]) => [to, new URL(from, ROOT).href]);

/** The demo's batch files and words on C:, beside the library's OWL.BAT. */
const DOS_WRITTEN = {
  // /STEP: the commander may step out of memory to run a program - the
  // program's lines in RUNPROG.BAT, written in the folder it was started
  // in - and is started again afterwards, in the same folders. A DOS
  // program then has all of DOS's memory, as from the prompt; OWL FLY III
  // needs it.
  'C/COMMANDR.BAT': `@echo off
cd \\DEMO
:again
call C:\\OWL.BAT C:\\DEMO\\COMMANDR\\COMMANDR.EXE /STEP C:\\DEMO A:\\
if not exist C:\\DEMO\\RUNPROG.BAT goto done
call C:\\DEMO\\RUNPROG.BAT
del C:\\DEMO\\RUNPROG.BAT
goto again
:done
cd \\
`,
  'C/DEMO.BAT': `@echo off
cd \\DEMO\\PASCAL
call C:\\OWL.BAT DEMO.EXE
cd \\
`,
  'C/CDEMO.BAT': `@echo off
cd \\DEMO\\C
call C:\\OWL.BAT DEMO.EXE
cd \\
`,
  'C/ASMDEMO.BAT': `@echo off
cd \\DEMO\\ASM
call C:\\OWL.BAT DEMO.COM
cd \\
`,
  'C/OWLFLY.BAT': `@echo off
cd \\GAMES\\OWLFLY3
OWLFLY3
cd \\
`,
  'C/README.TXT': `
  This PC runs in your browser: DOSBox, compiled to WebAssembly.

    COMMANDR   the file manager: C:\\DEMO on the left, floppy A: on the right
    DEMO       the OWLOSUI demo in Pascal; CDEMO in C, ASMDEMO in assembler
    OWLFLY     OWL FLY III, over the network

  Each of them draws with OWLOSRES: the same Rust core as the page around this
  window, built for DOS. A: and B: are folders of the browser's storage, DOS A
  Drive and DOS B Drive: put files there from the page and find them here.
  Right Ctrl gives the keyboard back to the page.

`,
};

/** What the DOS PC can start into, beside the library's prompt. */
const DOS_STARTS = {
  // `reread`: it shows a folder as it read it, and Ctrl+R reads it again -
  // pressed for the person when the page writes to a floppy.
  commander: { name: 'the commander', line: 'call C:\\COMMANDR.BAT', reread: true, label: '~C~ommander on DOS',
    hint: 'The file manager, in Pascal, on the same Rust core as this page - built for DOS' },
  demo: { name: 'the OWLOSUI demo', line: 'call C:\\DEMO.BAT', label: 'OWLOSUI ~d~emo on DOS', hint: 'This demo, written again in Pascal, on DOS' },
  owlfly: { name: 'OWL FLY III', line: 'call C:\\OWLFLY.BAT', network: true, label: 'Play OWL ~F~LY III',
    hint: 'A DOS flight game on the network: every player in the same room shares the sky' },
};

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
    this.browser = new BrowserStorage({ samples: SAMPLES });
    // Files in editors (lib/js/apps/documents.js): File > New and Open
    // from, and Save and Save as on the File menu while an editor is in
    // front. A new file's Save goes to this browser's storage.
    const origin = globalThis.location?.origin ?? 'http://localhost:8765';
    this.documents = new Documents(owl, {
      places: [
        { label: "~T~his browser's storage...", source: this.browser,
          hint: 'Files this browser keeps for this page, on this computer; nothing is uploaded',
          saveHint: 'Under a name and in a folder you choose, in this browser' },
        { label: "The ~s~erver's folder...", source: () => new ServerFolder(),
          hint: 'Examples/Web/files on the computer running RUN.CMD' },
        { label: 'A ~W~ebDAV folder...', hint: 'A folder shared the way a NAS or Nextcloud shares one; RUN.CMD shares one too',
          dav: { address: `${origin}/dav/`, about: 'WebDAV is how NAS boxes, Nextcloud and many servers share folders. The address ' +
            "below is RUN.CMD's own share of Examples/Web/files, the same files as the server's folder: leave it " +
            'to try, or type another.' } },
      ],
      defaultSource: this.browser,
      closeCmd: CmClose,
      newText: 'Type here. Ctrl+Z takes it back, Shift and the arrows select.\n' +
        "F2 saves it in this browser's storage.\n",
    });
    this.tools.push(this.documents);
    // The commander's drives: every place a page can keep files, and the
    // examples themselves to copy from.
    this.commander = new CommanderTool(owl, {
      sources: [new Repository(), this.browser, new ServerFolder(), new WebDavFolder(`${origin}/dav/`)],
      // A file opened from the commander fills the desktop, as a
      // commander's editor always has; F5 puts it back to a window.
      open: (name, text, source, path, options) => owl.zoom(this.documents.add(name, text, source, path, options)),
      closeCmd: CmClose,
      quietKeys: [{ cmd: CmHelp, key: 'F1' }, { cmd: CmClose, key: 'F3', alt: true }, { cmd: CmExit, key: 'x', alt: true }],
      // Enter on a DOS program: it runs on the DOS PC, in its window.
      run: (source, dir, name) => this.dos.runProgram(source, dir, name),
    });
    this.tools.push(this.commander);
    // The DOS PC (lib/js/apps/dos.js), its disk C: the library's toolkit,
    // and this repository's DOS examples - the commander first - and OWL
    // FLY III.
    // Its floppies are folders of this browser's storage, DOS A Drive and
    // DOS B Drive: the commander reaches them there.
    this.dos = new DosTool(owl, {
      commander: this.commander,
      storage: this.browser,
      open: (name, text, source, path, options) => this.documents.add(name, text, source, path, options),
      starts: DOS_STARTS,
      disk: DOS_DISK,
      bundles: [{ url: new URL('Examples/Web/dos/owlfly3_v17.jsdos', ROOT).href, to: 'GAMES/OWLFLY3' }],
      written: DOS_WRITTEN,
      // A program run from the page's commander gives way to DOS's.
      start: 'commander',
      returnTo: 'commander',
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
        // New and Open from. Save and Save as are not here: an editor
        // window brings them, above Exit, while it is in front. With no
        // file in front there is nothing to save, and the menu does not
        // offer it.
        ...this.documents.fileMenu(),
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
      { label: '~F3~ New', cmd: DocCm.New, key: 'F3' },
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
    for (const t of [this.documents, this.commander, this.dos, this.welcome, this.console]) if (t.owns(a)) { t.closeWindow(a); return; }
    const tool = this.tools.find(t => t.id === a);
    owl.close(a);
    if (tool) tool.closed();
    if (a === this.options) this.options = 0;
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
  }
}
