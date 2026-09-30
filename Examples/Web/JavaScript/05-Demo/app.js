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
//   Tools     Calculator (step 3's, imported as it is), Calendar, ASCII
//             table, Puzzle - tools.js, one class each.
//   Options   A dialog of check boxes and radio buttons; the colour dialog,
//             every role of the palette changed live; a ticked item.
//   Window    Size/Move, Zoom, Next, Previous, Close, List, Cascade, Tile:
//             the desktop's own verbs. Alt+1..9 reach numbered windows.
//   Help      About.
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

export const CmNew = 1, CmExit = 2;
export const CmCalc = 10, CmCalendar = 11, CmAscii = 12, CmPuzzle = 13;
export const CmOptions = 20, CmClock = 21, CmColors = 22;
export const CmNext = 30, CmZoom = 31, CmClose = 32, CmCascade = 33, CmTile = 34, CmPrevious = 35, CmList = 36, CmSizeMove = 37;
export const CmAbout = 40, CmHelp = 41;
export const CmOk = 60, CmCancel = 61, CmDismiss = 62;
export const CmOpenBrowser = 50, CmOpenServer = 51, CmOpenDav = 52, CmSave = 53;
export const CmFileOpen = 54, CmFileCancel = 55, CmDavConnect = 56, CmDavCancel = 57;

export class App {
  constructor(owl) {
    this.owl = owl;
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
    this.browser = new BrowserStorage();
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
        { label: '~S~ave', cmd: CmSave, shortcut: 'F2', hint: 'The file in front, back where it came from' },
        line(),
        { label: 'E~x~it', cmd: CmExit, shortcut: 'Alt+X', hint: 'Leave the program' }),
      sub('~T~ools',
        { label: '~C~alculator', cmd: CmCalc, hint: 'Arithmetic, trigonometry, hex and binary' },
        { label: 'Ca~l~endar', cmd: CmCalendar, hint: 'A month at a time; click a day' },
        { label: '~A~SCII table', cmd: CmAscii, hint: 'Every glyph of the font, by its number' },
        { label: '~P~uzzle', cmd: CmPuzzle, hint: 'The fifteen puzzle' }),
      sub('~O~ptions',
        { label: '~M~ouse...', cmd: CmOptions, hint: 'Check boxes and radio buttons, read back on OK' },
        { label: 'Co~l~ors...', cmd: CmColors, hint: "Every role's colour, changed live" },
        { label: '~C~lock on status line', cmd: CmClock, checked: this.clock, hint: 'A ticked option: on or off' }),
      sub('~W~indow',
        { label: '~S~ize/Move', cmd: CmSizeMove, shortcut: 'Ctrl+F5', hint: 'Arrows move the window, Shift+arrows resize it; Enter keeps, Esc puts back' },
        { label: '~Z~oom', cmd: CmZoom, shortcut: 'F5', hint: 'The window fills the desktop, or goes back to its size' },
        { label: '~N~ext', cmd: CmNext, shortcut: 'F6', hint: 'The front window goes to the back' },
        { label: '~P~revious', cmd: CmPrevious, shortcut: 'Shift+F6', hint: 'The window at the back comes to the front' },
        { label: '~C~lose', cmd: CmClose, shortcut: 'Alt+F3', hint: 'Close the front window' },
        { label: '~L~ist...', cmd: CmList, shortcut: 'Alt+0', hint: 'Every window by number; Enter brings one to the front' },
        line(),
        { label: 'C~a~scade', cmd: CmCascade, hint: 'The windows along the diagonal, every title showing' },
        { label: '~T~ile', cmd: CmTile, hint: 'The windows share the desktop with no overlap' }),
      sub('~H~elp',
        { label: '~A~bout', cmd: CmAbout, hint: 'What this program is and what draws it' }),
    );
    owl.statusLine(
      { label: '~F1~ Help', cmd: CmHelp, key: 'F1' },
      { label: '~F2~ Save', cmd: CmSave, key: 'F2' },
      { label: '~F3~ New', cmd: CmNew, key: 'F3' },
      { label: '~F5~ Zoom', cmd: CmZoom, key: 'F5' },
      { label: '~F6~ Next', cmd: CmNext, key: 'F6' },
      { label: '~Alt-F3~ Close', cmd: CmClose, key: 'F3', alt: true },
      { label: '~Alt-X~ Exit', cmd: CmExit, key: 'x', alt: true },
    );
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

  /** An editor window for a file; `source` and `path` say where Save puts it back. */
  addDoc(name, text, source, path) {
    const n = this.editors.length + 1;
    const title = source ? `${name} - ${source.title}` : name;
    const w = this.owl.window(title, 60, 16, { x: 2 + (n % 8), y: 1 + (n % 8), closeCmd: CmClose });
    // A DOS or Windows file ends its lines with CR LF. The editor wants LF
    // alone - a CR would show as a glyph - so the CRs come off here and go
    // back on in save(), and the file keeps the line ends it came with.
    const crlf = text.includes('\r\n');
    const t = this.owl.text(w, crlf ? text.replace(/\r\n/g, '\n') : text);
    // Everything the editor has, on an Edit menu of its own while this
    // window is in front: Find, Replace, Word wrap, Read only, Hex view,
    // Classic keys. The core runs all of it; nothing comes back here.
    this.owl.editor(t, Offer.All);
    this.editors.push(w);
    this.docs.set(w, { text: t, source, path, name, crlf });
  }

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

  /** File > Open from: the Open dialog, the source explained above the panel. */
  openFrom(source) {
    const owl = this.owl;
    this.closeOpen();
    const d = owl.window(`Open - ${source.title}`, 72, 21, { style: Style.ModalDialog, closeCmd: CmFileCancel });
    // The panel first: the core gives a window's keys to a file panel only
    // when it is the window's first part. The words about the source go in
    // the rows it leaves free above itself.
    const panel = owl.files(d, `${source.prefix}/*.*`, [], { top: 3 });
    owl.staticText(d, 2, 1, source.about, 66, 2);
    owl.buttons(d, { label: '~O~pen', cmd: CmFileOpen, default: true }, { label: '~C~ancel', cmd: CmFileCancel, cancel: true });
    this.open = { dialog: d, panel, source, dir: '/', entries: [] };
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

  /** A file into an editor window; the dialog closes. */
  async openFile(path) {
    const o = this.open;
    if (!o) return;
    try {
      const text = await o.source.read(path);
      this.closeOpen();
      this.addDoc(split(path).name, text, o.source, path);
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

  /** File > Save: back where it came from; a new file goes to this browser's storage. */
  save() {
    const doc = this.docs.get(this.owl.active());
    if (!doc) {
      this.tell('Save', 'The window in front is not a file. Open one with File > Open from, or make one with File > New.');
      return null;
    }
    const typed = this.owl.getText(doc.text);
    const text = doc.crlf ? typed.replace(/\r?\n/g, '\r\n') : typed;
    if (!doc.source) {
      doc.source = this.browser;
      doc.path = `/${doc.name}`;
    }
    return this.track((async () => {
      await doc.source.write(doc.path, text);
      this.tell('Saved', `${doc.name} is saved in ${doc.source.title}, as ${doc.path}. File > Open from finds it there.`);
    })());
  }

  /** File > Open from > A WebDAV folder: where the share is, first. */
  davAsk() {
    const owl = this.owl;
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
    this.closeDav();
    return this.openFrom(source);
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
      case CmAbout:
        this.tell('OWLOS UI Demo', 'A Turbo Vision-shaped toolkit with one portable core: this page is JavaScript, ' +
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
      case CmFileCancel: this.closeOpen(); return true;
      case CmFileOpen: {
        // The Open button: whatever is under the cursor.
        const [name] = this.open ? owl.markedNames(this.open.panel) : [];
        if (name) this.chosen(name);
        return true;
      }
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
