// Web, JavaScript, step 5 of 5 - everything the kit has, in one program, with a menu bar.
// Before: 04-Sokoban. The same demo on the desktop is Examples/Desktop/CSharp/05-OwlosDemo.
//
// The shape every text-mode toolkit's demo has had, because it is the right
// shape: a menu bar, a status line, a desktop, and small windows that each
// use one part of the kit.
//
//   File      New editor windows; Exit.
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

import { Style, sub, line } from '../../../../lib/js/owlosui.js';
import { CalculatorWindow } from '../03-Calculator/calculator.js';
import { CalendarWindow, AsciiTableWindow, PuzzleWindow, ColorsWindow } from './tools.js';

export const CmNew = 1, CmExit = 2;
export const CmCalc = 10, CmCalendar = 11, CmAscii = 12, CmPuzzle = 13;
export const CmOptions = 20, CmClock = 21, CmColors = 22;
export const CmNext = 30, CmZoom = 31, CmClose = 32, CmCascade = 33, CmTile = 34, CmPrevious = 35, CmList = 36, CmSizeMove = 37;
export const CmAbout = 40, CmHelp = 41;
export const CmOk = 60, CmCancel = 61, CmDismiss = 62;

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
    this.options = 0;
    this.box = 0;
    this.clock = true;

    // A hint is one line the status line shows while the cursor stands on
    // the item, in place of the keys.
    owl.menuBar(
      sub('~F~ile',
        { label: '~N~ew', cmd: CmNew, shortcut: 'F3', hint: 'An editor window of its own' },
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
    const w = this.owl.window(`UNTITLED${n}.TXT`, 50, 14, { x: 2 + n, y: 1 + n, closeCmd: CmClose });
    this.owl.text(w, 'Type here. Ctrl+Z takes it back, Shift and the arrows select.\n');
    this.editors.push(w);
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
    if (a === this.options) this.options = 0;
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
  }
}
