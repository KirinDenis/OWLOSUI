// Web, JavaScript, step 4 of 5 - a game: a canvas of cells the program draws itself.
// Before: 03-Calculator. Next: 05-Demo, every part of the kit in one program.
//
// BASE-Z 47, sixty warehouses of Sokoban - the same as
// Examples/Desktop/CSharp/04-Basez-Sokoban. What it adds to the ladder:
//
//  * A canvas. The board is not made of controls: the program works out
//    two cells per square - wall, floor, mark, box, keeper - and blits the
//    block onto a canvas view the core keeps and draws. A cell left
//    CLEAR_ATTR is not drawn at all, so outside the warehouse the window's
//    own blue shows through.
//
//  * Keys that change with the screen. On the list of levels the status
//    line says F1; in a game it binds the arrows, WASD, R, Z, N, P, Q and
//    Escape. A status line is not only a label: it is where keys are bound,
//    and the program swaps it when the meaning of a key changes.
//
//  * The rules have no screen. Warehouse knows walls, boxes and moves;
//    App only draws what Warehouse says and passes it the keys.

import { Style, attr, Color, CLEAR_ATTR } from '../../../../lib/js/owlosui.js';
import { LEVELS, W, H } from './levels.js';

const CmPlay = 1, CmExit = 2, CmMenu = 3, CmUp = 4, CmDown = 5, CmLeft = 6, CmRight = 7;
const CmRestart = 8, CmUndo = 9, CmNext = 10, CmPrev = 11, CmHelp = 12, CmDismiss = 13;

// Muted, as the original game had them.
const WALL = attr(Color.LightBlue, Color.Blue);
const FLOOR = attr(Color.Green, Color.Green);
const MARK = attr(Color.Brown, Color.Green);
const BOX = attr(Color.Brown, Color.Brown);
const PLACED = attr(Color.Brown, Color.Green);
const KEEPER = attr(Color.Red, Color.Green);

/** One warehouse: the squares as they are (cur), as they were without boxes and keeper (orig). */
export class Warehouse {
  static load(src) {
    const w = new Warehouse();
    w.cur = new Array(W * H).fill(' ');
    w.orig = new Array(W * H).fill(' ');
    w.hero = 0;
    w.moves = 0;
    w.goals = 0;
    src.split('\n').slice(0, H).forEach((line, y) => {
      [...line].slice(0, W).forEach((c, x) => {
        const i = y * W + x;
        w.cur[i] = c;
        if (c === '@') w.hero = i;
        else if (c !== '$') {
          w.orig[i] = c;
          if (c === '.') w.goals++;
        }
      });
    });
    return w;
  }

  copy() {
    const w = new Warehouse();
    Object.assign(w, this, { cur: [...this.cur], moves: 0, insideCache: undefined });
    return w;
  }

  /** How far right anything reaches, and how many rows have anything in them. */
  measure() {
    let right = 0, bottom = H;
    for (let y = 0; y < H; y++) {
      let r = 0;
      for (let x = W - 1; x >= 0; x--) if (this.cur[y * W + x] !== ' ') { r = x; break; }
      if (r !== 0) right = Math.max(right, r);
      else if (bottom > y) bottom = y;
    }
    return { right, rows: Math.max(bottom, 1) };
  }

  get onGoals() { return this.cur.filter((c, i) => c === '$' && this.orig[i] === '.').length; }
  get won() { return this.goals > 0 && this.cur.every((c, i) => c !== '$' || this.orig[i] === '.'); }

  /** The squares the keeper could walk to: the inside, as a flood fill from the keeper. */
  inside() {
    if (this.insideCache) return this.insideCache;
    const seen = new Array(W * H).fill(false);
    const todo = [this.hero];
    seen[this.hero] = true;
    while (todo.length) {
      const i = todo.pop();
      const r = Math.floor(i / W), c = i % W;
      for (const [dr, dc] of [[-1, 0], [1, 0], [0, -1], [0, 1]]) {
        const nr = r + dr, nc = c + dc;
        if (nr < 0 || nc < 0 || nr >= H || nc >= W) continue;
        const j = nr * W + nc;
        if (seen[j] || this.orig[j] === '#') continue;
        seen[j] = true;
        todo.push(j);
      }
    }
    return (this.insideCache = seen);
  }

  /** A square as two glyphs and a colour. */
  cell(i) {
    const c = this.cur[i];
    switch (c) {
      case '#': return ['▓▓', WALL];                                      // ▓▓ stone
      case '$': return this.orig[i] === '.' ? ['▒▒', PLACED] : ['  ', BOX]; // ▒▒ on a mark
      case '@': return ['▐▌', KEEPER];                                    // ▐▌ a figure
      case '.': return ['■■', MARK];                                      // ■■ the mark
      default: return this.inside()[i] ? ['  ', FLOOR] : ['\0\0', CLEAR_ATTR];      // outside: clear
    }
  }

  /** One step; pushes a box if there is one that can go. Returns whether anything moved. */
  tryMove(drow, dcol, undo) {
    const r = Math.floor(this.hero / W) + drow, c = (this.hero % W) + dcol;
    if (r < 0 || r >= H || c < 0 || c >= W) return false;
    const ni = r * W + c;
    const dest = this.cur[ni];
    let bi = -1;
    if (dest === '$') {
      const br = r + drow, bc = c + dcol;
      if (br < 0 || br >= H || bc < 0 || bc >= W) return false;
      bi = br * W + bc;
      if (this.cur[bi] !== ' ' && this.cur[bi] !== '.') return false;
    } else if (dest !== ' ' && dest !== '.') return false;
    undo.push({ cur: [...this.cur], hero: this.hero, moves: this.moves });
    if (bi >= 0) this.cur[bi] = '$';
    this.cur[this.hero] = this.orig[this.hero];
    this.cur[ni] = '@';
    this.hero = ni;
    this.moves++;
    return true;
  }
}

export class App {
  constructor(owl) {
    this.owl = owl;
    this.maps = LEVELS.map(Warehouse.load);
    this.undo = [];
    this.level = 0;
    this.window = 0;
    this.box = 0;
    this.showMenu();
  }

  showMenu() {
    const owl = this.owl;
    this.menu = owl.window('BASE-Z 47', 44, 20, { style: Style.Dialog, closeCmd: CmExit });
    owl.staticText(this.menu, 1, 0, 'Sixty warehouses. Enter plays.', 40);
    this.menuList = owl.list(this.menu, 0, 1, 40, 14, this.maps.map((_, i) => `Level ${String(i + 1).padStart(2)}`));
    owl.buttons(this.menu, { label: '~P~lay', cmd: CmPlay, default: true }, ['~Q~uit', CmExit]);
    owl.statusLine({ label: '~F1~ Help', cmd: CmHelp, key: 'F1' });
  }

  gameKeys() {
    this.owl.statusLine(
      { label: '^', cmd: CmUp, key: 'ArrowUp' }, { label: 'v', cmd: CmDown, key: 'ArrowDown' },
      { label: '<', cmd: CmLeft, key: 'ArrowLeft' }, { label: '>', cmd: CmRight, key: 'ArrowRight' },
      { label: 'W', cmd: CmUp, key: 'w' }, { label: 'A', cmd: CmLeft, key: 'a' },
      { label: 'S', cmd: CmDown, key: 's' }, { label: 'D', cmd: CmRight, key: 'd' },
      { label: '~R~estart', cmd: CmRestart, key: 'r' }, { label: '~Z~ undo', cmd: CmUndo, key: 'z' },
      { label: '~N~ext', cmd: CmNext, key: 'n' }, { label: '~P~rev', cmd: CmPrev, key: 'p' },
      { label: '~Q~uit', cmd: CmMenu, key: 'q' }, { label: 'Esc', cmd: CmMenu, key: 'Escape' },
      { label: '~F1~', cmd: CmHelp, key: 'F1' },
    );
  }

  startLevel() {
    const owl = this.owl;
    this.undo = [];
    this.play = this.maps[this.level].copy();
    if (this.window) owl.close(this.window);
    // The whole desktop above the status line.
    const winW = Math.max(owl.width, 20), winH = Math.max(owl.height - 1, 8);
    this.window = owl.window('BASE-Z 47', winW, winH, { x: 0, y: 0, closeCmd: CmMenu });
    const clientW = winW - 2, clientH = winH - 2;
    const { right, rows } = this.play.measure();
    this.cols = right + 1;
    this.rows = rows;
    // Centred, in whole squares.
    const ox = Math.max(0, (Math.floor(clientW / 2 / 2) - Math.floor(right / 2)) * 2);
    const oy = Math.max(1, Math.floor(clientH / 2) - Math.floor(rows / 2));
    this.caption = owl.staticText(this.window, 1, Math.max(0, oy - 1), '', clientW - 2);
    this.board = owl.canvas(this.window, ox, oy, this.cols * 2, rows);
    this.redraw();
  }

  redraw() {
    const p = this.play;
    this.owl.setText(this.caption, `Level ${this.level + 1}  moves ${p.moves}  ${p.onGoals}/${p.goals}`);
    const w = this.cols * 2;
    const chars = new Array(w * this.rows);
    const attrs = new Array(w * this.rows);
    for (let y = 0; y < this.rows; y++) {
      for (let x = 0; x < this.cols; x++) {
        const [g, a] = p.cell(y * W + x);
        const at = y * w + x * 2;
        chars[at] = g[0];
        chars[at + 1] = g[1];
        attrs[at] = attrs[at + 1] = a;
      }
    }
    this.owl.blit(this.board, 0, 0, w, this.rows, chars, attrs);
  }

  move(dx, dy) {
    if (!this.window || !this.play.tryMove(dy, dx, this.undo)) return;
    if (!this.play.won) return this.redraw();
    if (this.level + 1 < this.maps.length) {
      this.level++;
      this.startLevel();
    } else {
      this.finished = true;
      this.tell('All 60 levels are clear.');
    }
  }

  tell(text) {
    this.closeBox();
    this.box = this.owl.messageBox('BASE-Z 47', text, ['~O~K', CmDismiss]);
  }

  closeBox() {
    if (this.box) this.owl.close(this.box);
    this.box = 0;
  }

  closeGame() {
    if (this.window) this.owl.close(this.window);
    this.window = 0;
  }

  onCommand(cmd) {
    const owl = this.owl;
    if (this.box && cmd !== CmDismiss && cmd !== CmExit) return true;
    switch (cmd) {
      case CmHelp:
        this.tell('Push every box onto a mark. A box can only be pushed, and only one at a time. Arrows or WASD. ' +
          'R restarts, Z takes the step back, N and P change the level. Q or Esc returns to the list.');
        return true;
      case CmDismiss:
        this.closeBox();
        if (this.finished) {
          this.finished = false;
          this.closeGame();
          this.showMenu();
        }
        return true;
      case CmPlay:
        this.level = Math.min(owl.current(this.menuList), this.maps.length - 1);
        owl.close(this.menu);
        this.menu = 0;
        this.startLevel();
        this.gameKeys();
        return true;
      case CmMenu:
        this.closeGame();
        this.showMenu();
        return true;
      case CmExit: return false;
      case CmUp: this.move(0, -1); return true;
      case CmDown: this.move(0, 1); return true;
      case CmLeft: this.move(-1, 0); return true;
      case CmRight: this.move(1, 0); return true;
      case CmRestart: if (this.window) this.startLevel(); return true;
      case CmUndo: {
        const shot = this.undo.pop();
        if (this.window && shot) {
          Object.assign(this.play, shot);
          this.redraw();
        }
        return true;
      }
      case CmNext: if (this.window && this.level + 1 < this.maps.length) { this.level++; this.startLevel(); } return true;
      case CmPrev: if (this.window && this.level > 0) { this.level--; this.startLevel(); } return true;
      default: return true;
    }
  }
}
