// The demo's small tools, each a class that owns a range of command numbers
// and has the same four verbs - show, onCommand (if handles(cmd)), poll,
// closed - so the demo only routes. The same tools as the C# demo's
// Calendar/, AsciiTable/, Puzzle/ and Colors/ folders.

import { Style, attr, Color } from '../../../../lib/js/owlosui.js';

// ------------------------------------------------------------- calendar

const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
const WEEKDAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
const today = () => { const d = new Date(); return new Date(d.getFullYear(), d.getMonth(), d.getDate()); };
const sameDay = (a, b) => a && b && a.getTime() === b.getTime();

/** A month on a canvas: day names, weeks, today green, the chosen day cyan, weekends red. */
export class CalendarWindow {
  static CmFirst = 160; static CmLast = 169;
  static CmPrev = 160; static CmNext = 161; static CmToday = 162;
  static CellW = 4; static CellH = 2;

  constructor(owl, closeCmd) {
    this.owl = owl;
    this.closeCmd = closeCmd;
    this.id = 0;
    const t = today();
    this.month = new Date(t.getFullYear(), t.getMonth(), 1);
    this.selected = null;
  }

  show() {
    const owl = this.owl;
    if (this.id) { owl.activate(this.id); return; }
    this.id = owl.window('Calendar', 40, 21, { style: Style.Dialog, closeCmd: this.closeCmd });
    this.title = owl.staticText(this.id, 5, 1, '', 30);
    this.grid = owl.canvas(this.id, 5, 2, 28, 13);
    this.caption = owl.staticText(this.id, 1, 16, '', 36);
    owl.buttons(this.id, ['~<~ Prev', CalendarWindow.CmPrev], ['~T~oday', CalendarWindow.CmToday], ['~N~ext ~>~', CalendarWindow.CmNext]);
    this.draw();
  }

  handles(cmd) { return this.id !== 0 && cmd >= CalendarWindow.CmFirst && cmd <= CalendarWindow.CmLast; }

  onCommand(cmd) {
    const m = this.month;
    if (cmd === CalendarWindow.CmPrev) { this.month = new Date(m.getFullYear(), m.getMonth() - 1, 1); this.selected = null; }
    else if (cmd === CalendarWindow.CmNext) { this.month = new Date(m.getFullYear(), m.getMonth() + 1, 1); this.selected = null; }
    else if (cmd === CalendarWindow.CmToday) { const t = today(); this.month = new Date(t.getFullYear(), t.getMonth(), 1); this.selected = t; }
    else return;
    this.draw();
  }

  /** A click on a day chooses it. */
  poll() {
    if (!this.id) return;
    const p = this.owl.canvasClick(this.grid);
    if (!p || p.y < 1) return;
    const slot = Math.floor((p.y - 1) / CalendarWindow.CellH) * 7 + Math.floor(p.x / CalendarWindow.CellW);
    const day = slot - this.month.getDay() + 1;
    const days = new Date(this.month.getFullYear(), this.month.getMonth() + 1, 0).getDate();
    if (day < 1 || day > days) return;
    this.selected = new Date(this.month.getFullYear(), this.month.getMonth(), day);
    this.draw();
  }

  draw() {
    const owl = this.owl, m = this.month, s = this.selected;
    owl.setText(this.title, `${MONTHS[m.getMonth()]} ${m.getFullYear()}`);
    owl.setText(this.caption, s ? `${WEEKDAYS[s.getDay()]}, ${s.getDate()} ${MONTHS[s.getMonth()]} ${s.getFullYear()}` : '');
    const W = 28, H = 13;
    const plain = attr(Color.Black, Color.LightGray), names = attr(Color.Blue, Color.LightGray);
    const weekend = attr(Color.Red, Color.LightGray), todayA = attr(Color.White, Color.Green);
    const chosen = attr(Color.Black, Color.Cyan), both = attr(Color.Yellow, Color.Green);
    const text = new Array(W * H).fill(' ');
    const attrs = new Array(W * H).fill(plain);
    ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'].forEach((n, c) => {
      text[c * 4 + 1] = n[0];
      text[c * 4 + 2] = n[1];
      attrs[c * 4 + 1] = attrs[c * 4 + 2] = c === 0 || c === 6 ? weekend : names;
    });
    const first = m.getDay(), days = new Date(m.getFullYear(), m.getMonth() + 1, 0).getDate(), t = today();
    for (let d = 1; d <= days; d++) {
      const slot = first + d - 1, col = slot % 7, row = Math.floor(slot / 7);
      const date = new Date(m.getFullYear(), m.getMonth(), d);
      const isToday = sameDay(date, t), isChosen = sameDay(date, s);
      const a = isToday && isChosen ? both : isToday ? todayA : isChosen ? chosen : col === 0 || col === 6 ? weekend : plain;
      const label = String(d).padStart(2);
      for (let dy = 0; dy < 2; dy++) {
        for (let dx = 0; dx < 4; dx++) {
          const at = (1 + row * 2 + dy) * W + col * 4 + dx;
          text[at] = dy === 0 && dx >= 1 && dx <= 2 ? label[dx - 1] : ' ';
          attrs[at] = a;
        }
      }
    }
    owl.blit(this.grid, 0, 0, W, H, text, attrs);
  }

  closed() { this.id = 0; }
}

// ---------------------------------------------------------- ASCII table

/** Every glyph below 256, sixteen by sixteen, with hex headings; a click names one in every base. */
export class AsciiTableWindow {
  constructor(owl, closeCmd) {
    this.owl = owl;
    this.closeCmd = closeCmd;
    this.id = 0;
    this.picked = null;
  }

  show() {
    const owl = this.owl;
    if (this.id) { owl.activate(this.id); return; }
    this.id = owl.window('ASCII table', 55, 22, { style: Style.Dialog, closeCmd: this.closeCmd });
    this.grid = owl.canvas(this.id, 1, 1, 51, 17);
    this.caption = owl.staticText(this.id, 1, 19, 'Click a glyph.', 51);
    this.picked = null;
    this.draw();
  }

  handles() { return false; }
  onCommand() {}

  poll() {
    if (!this.id) return;
    const p = this.owl.canvasClick(this.grid);
    if (!p || p.y < 1 || p.x < 3) return;
    const code = (p.y - 1) * 16 + Math.floor((p.x - 3) / 3);
    if (code >= 256) return;
    this.picked = code;
    this.draw();
    this.owl.setText(this.caption, `Glyph ${code}  dec ${code}  hex ${code.toString(16).toUpperCase().padStart(2, '0')}  ` +
      `oct ${code.toString(8)}  bin ${code.toString(2).padStart(8, '0')}`);
  }

  draw() {
    const W = 51, H = 17;
    const plain = attr(Color.Black, Color.LightGray), head = attr(Color.Blue, Color.LightGray), lit = attr(Color.White, Color.Cyan);
    const text = new Array(W * H).fill(' ');
    const attrs = new Array(W * H).fill(plain);
    for (let c = 0; c < 16; c++) { text[3 + c * 3 + 1] = '0123456789ABCDEF'[c]; attrs[3 + c * 3 + 1] = head; }
    for (let r = 0; r < 16; r++) {
      const y = 1 + r, label = (r * 16).toString(16).toUpperCase().padStart(2, '0');
      text[y * W] = label[0];
      text[y * W + 1] = label[1];
      attrs[y * W] = attrs[y * W + 1] = head;
      for (let c = 0; c < 16; c++) {
        const code = r * 16 + c, x = 3 + c * 3;
        // The glyph as the session's font has it: a face at 1, a line at C4.
        text[y * W + x + 1] = code === 0 ? ' ' : this.owl.glyphs[code] ?? '?';
        if (this.picked === code) for (let dx = 0; dx < 3; dx++) attrs[y * W + x + dx] = lit;
      }
    }
    this.owl.blit(this.grid, 0, 0, W, H, text, attrs);
  }

  closed() { this.id = 0; }
}

// --------------------------------------------------------------- puzzle

/** The fifteen puzzle without a screen: a tile beside the hole slides into it. */
export class Board {
  constructor() { this.reset(); }

  reset() {
    this.tiles = Array.from({ length: 16 }, (_, i) => (i + 1) % 16);
    this.moves = 0;
  }

  get hole() { return this.tiles.indexOf(0); }
  get solved() { return this.tiles.slice(0, 15).every((t, i) => t === i + 1); }

  slide(at) {
    if (at < 0 || at >= 16 || this.tiles[at] === 0) return false;
    const h = this.hole;
    if (Math.abs(Math.floor(at / 4) - Math.floor(h / 4)) + Math.abs((at % 4) - (h % 4)) !== 1) return false;
    [this.tiles[at], this.tiles[h]] = [0, this.tiles[at]];
    this.moves++;
    return true;
  }

  /** Two hundred legal slides from the solved board: always solvable. Seeded, so a test knows the board. */
  scramble(seed = 47) {
    this.reset();
    let s = seed >>> 0;
    const next = () => { s = (s * 1664525 + 1013904223) >>> 0; return s; };
    for (let n = 0; n < 200; n++) {
      const h = this.hole, r = Math.floor(h / 4), c = h % 4, moves = [];
      if (r > 0) moves.push(h - 4);
      if (r < 3) moves.push(h + 4);
      if (c > 0) moves.push(h - 1);
      if (c < 3) moves.push(h + 1);
      this.slide(moves[next() % moves.length]);
    }
    this.moves = 0;
  }
}

/** The board on a canvas, tiles six by three; clicks slide them. */
export class PuzzleWindow {
  static CmFirst = 180; static CmLast = 189; static CmScramble = 180;

  constructor(owl, closeCmd) {
    this.owl = owl;
    this.closeCmd = closeCmd;
    this.id = 0;
    this.board = new Board();
  }

  show() {
    const owl = this.owl;
    if (this.id) { owl.activate(this.id); return; }
    this.id = owl.window('Puzzle', 36, 19, { style: Style.Dialog, closeCmd: this.closeCmd });
    this.grid = owl.canvas(this.id, 5, 1, 24, 12);
    this.caption = owl.staticText(this.id, 1, 14, '', 32);
    owl.buttons(this.id, ['~S~cramble', PuzzleWindow.CmScramble]);
    this.board.scramble();
    this.draw();
  }

  handles(cmd) { return this.id !== 0 && cmd >= PuzzleWindow.CmFirst && cmd <= PuzzleWindow.CmLast; }

  onCommand(cmd) {
    if (cmd !== PuzzleWindow.CmScramble) return;
    this.board.scramble(Date.now() & 0xFFFF);
    this.draw();
  }

  poll() {
    if (!this.id) return;
    const p = this.owl.canvasClick(this.grid);
    if (!p) return;
    const col = Math.floor(p.x / 6), row = Math.floor(p.y / 3);
    if (col < 4 && row < 4 && this.board.slide(row * 4 + col)) this.draw();
  }

  draw() {
    const W = 24, H = 12, b = this.board, solved = b.solved;
    const hole = attr(Color.LightGray, Color.LightGray), even = attr(Color.White, Color.Blue);
    const odd = attr(Color.White, Color.Cyan), done = attr(Color.White, Color.Green);
    const text = new Array(W * H).fill(' ');
    const attrs = new Array(W * H).fill(hole);
    b.tiles.forEach((tile, i) => {
      const r = Math.floor(i / 4), c = i % 4;
      const a = tile === 0 ? hole : solved ? done : tile % 2 === 0 ? even : odd;
      const label = tile === 0 ? '' : String(tile), lx = Math.floor((6 - label.length) / 2);
      for (let dy = 0; dy < 3; dy++) {
        for (let dx = 0; dx < 6; dx++) {
          const at = (r * 3 + dy) * W + c * 6 + dx, k = dx - lx;
          text[at] = dy === 1 && k >= 0 && k < label.length ? label[k] : ' ';
          attrs[at] = a;
        }
      }
    });
    this.owl.blit(this.grid, 0, 0, W, H, text, attrs);
    this.owl.setText(this.caption, solved ? `Solved in ${b.moves} moves!` : `Moves: ${b.moves}`);
  }

  closed() { this.id = 0; }
}

// --------------------------------------------------------------- colours

/** Every role of the palette on the left, the sixteen colours on the right: a click recolours it live. */
export class ColorsWindow {
  static CmFirst = 210; static CmLast = 219; static CmOk = 210; static CmCancel = 211;

  constructor(owl) {
    this.owl = owl;
    this.id = 0;
  }

  show() {
    const owl = this.owl;
    if (this.id) { owl.activate(this.id); return; }
    this.entries = owl.palette();
    this.before = this.entries.map(e => e.attr);
    this.id = owl.window('Colors', 66, 20, { style: Style.ModalDialog, closeCmd: ColorsWindow.CmCancel });
    this.list = owl.list(this.id, 1, 1, 34, 15, this.entries.map(e => `${e.group}: ${e.name}`));
    owl.staticText(this.id, 38, 1, 'Foreground');
    this.fore = owl.canvas(this.id, 38, 2, 24, 2);
    owl.staticText(this.id, 38, 5, 'Background');
    this.back = owl.canvas(this.id, 38, 6, 24, 2);
    this.sample = owl.canvas(this.id, 38, 10, 22, 1);
    owl.buttons(this.id, { label: '~O~K', cmd: ColorsWindow.CmOk, default: true }, { label: '~C~ancel', cmd: ColorsWindow.CmCancel, cancel: true });
    this.shown = -1;
    this.poll();
  }

  handles(cmd) { return this.id !== 0 && cmd >= ColorsWindow.CmFirst && cmd <= ColorsWindow.CmLast; }

  onCommand(cmd) {
    // Cancel puts back what was there when the dialog opened.
    if (cmd === ColorsWindow.CmCancel) this.before.forEach((a, i) => { if (this.entries[i].attr !== a) this.owl.setColor(i, a); });
    this.closed();
  }

  poll() {
    if (!this.id) return;
    const ix = this.owl.current(this.list);
    if (ix !== this.shown) { this.shown = ix; this.draw(); }
    const f = this.owl.canvasClick(this.fore);
    if (f) this.pick(f, true);
    const b = this.owl.canvasClick(this.back);
    if (b) this.pick(b, false);
  }

  pick(at, foreground) {
    const e = this.entries[this.shown];
    if (!e) return;
    const colour = at.y * 8 + Math.floor(at.x / 3);
    if (colour < 0 || colour > 15) return;
    e.attr = foreground ? (e.attr & 0xF0) | colour : (colour << 4) | (e.attr & 0x0F);
    this.owl.setColor(this.shown, e.attr);
    this.draw();
  }

  draw() {
    const e = this.entries[this.shown];
    if (!e) return;
    this.grid(this.fore, e.attr & 15);
    this.grid(this.back, e.attr >> 4);
    const sample = ' Sample text          ';
    this.owl.blit(this.sample, 0, 0, sample.length, 1, sample, new Array(sample.length).fill(e.attr));
  }

  grid(canvas, chosen) {
    const text = new Array(48).fill(' '), attrs = new Array(48);
    for (let c = 0; c < 16; c++) {
      const x0 = (c % 8) * 3, y = Math.floor(c / 8), ink = c < 8 ? 15 : 0;
      for (let dx = 0; dx < 3; dx++) {
        text[y * 24 + x0 + dx] = dx === 1 && c === chosen ? 'X' : ' ';
        attrs[y * 24 + x0 + dx] = (c << 4) | ink;
      }
    }
    this.owl.blit(canvas, 0, 0, 24, 2, text, attrs);
  }

  closed() {
    if (this.id) this.owl.close(this.id);
    this.id = 0;
  }
}
