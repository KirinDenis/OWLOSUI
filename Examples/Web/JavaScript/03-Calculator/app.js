// Web, JavaScript, step 3 of 5 - a calculator: a keypad of real buttons, modes, and arithmetic kept apart.
// Before: 02-Notes. Next: 04-Sokoban, a game drawn cell by cell on a canvas.
//
// Three files, and the split is the lesson:
//
//   calc-engine.js   the arithmetic - a string in, a string out, no screen.
//                    It can be tested, or used by anything else, alone.
//   calculator.js    the window - which key puts what into the display.
//                    It owns its command numbers, so another program (the
//                    demo, step 5) can take it as it is.
//   app.js           this: a desktop with a status line and the calculator.
//
// Type the sum, or click it: the keypad is not a Tab stop and a click does
// not take the caret away, so both go into the display. Enter is =,
// Escape is C. Switch to Hex and the digits A to F come alive.

import { CalculatorWindow } from './calculator.js';

export const CmExit = 1, CmClose = 2;

export class App {
  constructor(owl) {
    this.owl = owl;
    owl.statusLine(
      { label: '~Alt-X~ Exit', cmd: CmExit, key: 'x', alt: true },
      { label: '~Enter~ =', cmd: 0 },
      { label: '~Esc~ Clear', cmd: 0 },
    );
    this.calc = new CalculatorWindow(owl, CmClose);
    this.calc.show();
  }

  onCommand(cmd) {
    if (cmd === CmExit || cmd === CmClose) return false;
    if (this.calc.handles(cmd)) this.calc.onCommand(cmd);
    return true;
  }

  poll() { this.calc.poll(); }
}
