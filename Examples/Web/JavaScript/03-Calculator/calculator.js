// The calculator's window: a display, a keypad of real buttons, and the
// modes down the side. The arithmetic is in calc-engine.js; this file only
// decides what a key puts into the display and when to ask the engine.
// The same window as the C# demo's, and liftable the same way: it owns a
// range of command numbers and answers handles(cmd) for them. The web demo
// (05-Demo) imports it from here.
//
//   C  MC  MR  M+  (   )        ( ) Dec
//  sin cos tan  √  x²  ^        ( ) Hex
//   7   8   9   A   B  /        ( ) Bin
//   4   5   6   C   D  *        ( ) Oct
//   1   2   3   E   F  -
//   0   .   ±  1/x  =  +        [ ] Radians
//
// The keys are buttons, not a picture of buttons: they go down when
// pressed, and a click does not take the caret from the display, so the
// keys can be typed as well - Enter is =, Escape is C. Digits a base does
// not have are disabled. C is red: the one key that throws work away.

import { Style } from '../../../../lib/js/owlosui.js';
import { CalcEngine, CalcError, Base } from './calc-engine.js';

const CmClear = 100, CmMemClear = 101, CmMemRecall = 102, CmMemAdd = 103, CmEquals = 104, CmNegate = 105, CmInverse = 106;
/** Keys that type into the display: CmKey + their index in KEYS. */
const CmKey = 110;

/** What a text key types, and its colour. The order is the order of the commands. */
const KEYS = [
  ['(', '(', 'accent'], [')', ')', 'accent'],
  ['sin', 'sin(', 'accent'], ['cos', 'cos(', 'accent'], ['tan', 'tan(', 'accent'], ['√', 'sqrt(', 'accent'],
  ['x²', '^2', 'accent'], ['^', '^', 'accent'],
  ['7', '7'], ['8', '8'], ['9', '9'], ['A', 'A'], ['B', 'B'], ['/', '/', 'accent'],
  ['4', '4'], ['5', '5'], ['6', '6'], ['C', 'C'], ['D', 'D'], ['*', '*', 'accent'],
  ['1', '1'], ['2', '2'], ['3', '3'], ['E', 'E'], ['F', 'F'], ['-', '-', 'accent'],
  ['0', '0'], ['.', '.'],
  ['+', '+', 'accent'],
];

const ERRORS = new Set(['Error', 'Divide by zero', 'Invalid input', 'Overflow']);

/** Three characters wide, the label in the middle, so the columns line up. */
const pad = label => (label.length === 1 ? ` ${label} ` : label.length === 2 ? ` ${label}` : label);

export class CalculatorWindow {
  static CmFirst = 100;
  static CmLast = 159;

  constructor(owl, closeCmd) {
    this.owl = owl;
    this.closeCmd = closeCmd;
    this.id = 0;
    this.engine = new CalcEngine();
  }

  /** Open it, or bring it to the front. */
  show() {
    const owl = this.owl;
    if (this.id) { owl.activate(this.id); return; }
    this.id = owl.window('Calculator', 70, 18, { style: Style.Dialog, closeCmd: this.closeCmd });
    // A row of air under the title, then the display.
    this.display = owl.input(this.id, 1, 1, 52, '', '');
    this.memoryMark = owl.staticText(this.id, 56, 1, ' ', 1);
    this.keyPlaces = [];
    const text = ix => ({ label: pad(KEYS[ix][0]), cmd: CmKey + ix, style: KEYS[ix][2] ?? 'normal' });
    const key = (label, cmd, style, dflt = false) => ({ label: pad(label), cmd, style, default: dflt });
    this.row(3, { label: pad('C'), cmd: CmClear, style: 'danger', cancel: true },
      key('MC', CmMemClear, 'accent'), key('MR', CmMemRecall, 'accent'), key('M+', CmMemAdd, 'accent'), text(0), text(1));
    this.row(5, text(2), text(3), text(4), text(5), text(6), text(7));
    this.row(7, text(8), text(9), text(10), text(11), text(12), text(13));
    this.row(9, text(14), text(15), text(16), text(17), text(18), text(19));
    this.row(11, text(20), text(21), text(22), text(23), text(24), text(25));
    this.row(13, text(26), text(27), key('±', CmNegate, 'accent'), key('1/x', CmInverse, 'accent'), key('=', CmEquals, 'accent', true), text(28));
    // The modes: radio buttons for the base; ticked Radians, else degrees.
    this.baseRadio = owl.cluster(this.id, 56, 3, 12, ['~D~ec', '~H~ex', '~B~in', '~O~ct'], { single: true });
    this.radiansCheck = owl.cluster(this.id, 56, 8, 12, ['~R~adians']);
    this.shownBase = Base.Dec;
    this.shownRadians = false;
    this.engine.base = Base.Dec;
    this.engine.degrees = true;
    this.enableDigits();
  }

  /** One row of the keypad: not a Tab stop, so typing goes on to the display. */
  row(y, ...buttons) {
    const row = this.owl.buttonRow(this.id, 1, y, buttons, { selectable: false });
    buttons.forEach((b, i) => { if (b.cmd >= CmKey && b.cmd < CmKey + KEYS.length) this.keyPlaces[b.cmd - CmKey] = [row, i]; });
  }

  handles(cmd) { return this.id !== 0 && cmd >= CalculatorWindow.CmFirst && cmd <= CalculatorWindow.CmLast; }

  onCommand(cmd) {
    if (!this.id) return;
    const owl = this.owl, e = this.engine;
    let text = owl.getText(this.display);
    switch (cmd) {
      case CmClear: owl.setText(this.display, ''); break;
      case CmEquals: owl.setText(this.display, e.evaluate(text)); break;
      case CmNegate:
        if (text) owl.setText(this.display, text.startsWith('-') ? text.slice(1) : '-' + text);
        break;
      case CmInverse:
        if (text) owl.setText(this.display, `1/(${text})`);
        break;
      case CmMemClear: e.memory = 0; this.showMemory(); break;
      case CmMemRecall: owl.setText(this.display, text + e.format(e.memory)); break;
      case CmMemAdd:
        try { e.memory += e.eval(text); this.showMemory(); } catch (err) {
          if (!(err instanceof CalcError)) throw err;
          owl.setText(this.display, err.message);
        }
        break;
      default:
        if (cmd >= CmKey && cmd < CmKey + KEYS.length) {
          if (ERRORS.has(text)) text = '';
          owl.setText(this.display, text + KEYS[cmd - CmKey][1]);
        }
    }
  }

  showMemory() { this.owl.setText(this.memoryMark, this.engine.hasMemory ? 'M' : ' '); }

  /** The modes are clusters, which send nothing: looked at after every input. */
  poll() {
    if (!this.id) return;
    const owl = this.owl, e = this.engine;
    // The one with the dot, not the one under the cursor.
    const which = owl.clusterState(this.baseRadio).on.indexOf(true);
    const wanted = [Base.Dec, Base.Hex, Base.Bin, Base.Oct][Math.max(0, which)];
    if (wanted !== this.shownBase) {
      // What is on the display, converted, if it is a number.
      const text = owl.getText(this.display);
      let value = null;
      if (text) { try { value = e.eval(text); } catch { /* not a number: left as it is */ } }
      e.base = wanted;
      this.shownBase = wanted;
      if (value !== null) {
        try { owl.setText(this.display, e.format(value)); } catch (err) { owl.setText(this.display, err.message); }
      }
      this.enableDigits();
      owl.focus(this.display);
    }
    const radians = owl.clusterState(this.radiansCheck).on[0] === true;
    e.degrees = !radians;
    if (radians !== this.shownRadians) {
      this.shownRadians = radians;
      owl.focus(this.display);
    }
  }

  /** Only the digits of the current base can be pressed; the point only in decimal. */
  enableDigits() {
    KEYS.forEach(([label], i) => {
      if (label.length !== 1) return;
      let on;
      if (label === '.') on = this.engine.base === Base.Dec;
      else if (/[0-9A-F]/.test(label)) on = this.engine.isDigit(label);
      else return;
      const [row, index] = this.keyPlaces[i];
      this.owl.enableButton(row, index, on);
    });
  }

  closed() { this.id = 0; }

  /** What the display says, for a test. */
  get text() { return this.id ? this.owl.getText(this.display) : ''; }
}
