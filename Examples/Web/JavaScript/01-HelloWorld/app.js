// Web, JavaScript, step 1 of 5 - a window, words, a button: the whole shape of a program.
// Next: 02-Notes, an editor that keeps its text and asks before you leave.
//
// The smallest OWLOSUI program there is, and the same one as
// Examples/Desktop/CSharp/01-HelloWorld line for line.
//
// What is worth noticing:
//
//  * Nothing here draws. The core - a WebAssembly module, owlosui-wire.wasm
//    - decides what every cell looks like: colours, frames, shadows, where
//    the button sits. This program says what it wants and hears which
//    button was pressed. owlosui.js copies the cells onto the canvas.
//
//  * There are no callbacks. A button does not have an onclick; it has a
//    number, and onCommand is handed that number when it is pressed. The
//    same numbers cross a pipe to C# and, one day, a DOS interrupt.
//
//  * Handles are numbers too: window() returns one and every other call
//    takes it.

import { Style } from '../../../../lib/js/owlosui.js';

// Command numbers are the program's own. Any number but 0, which means
// "nothing happened".
export const CmOk = 1;

export class App {
  constructor(owl) {
    // 40 cells wide, 9 tall, centred (the default). Style.Dialog is grey,
    // fixed in size and closable: the look of something that asks.
    const w = owl.window('Hello', 40, 9, { style: Style.Dialog });
    // Placed by hand, in cells, inside the frame: column 2, row 1.
    owl.staticText(w, 2, 1, 'Hello, world!');
    // Wrapped at 34 cells into 2 rows.
    owl.staticText(w, 2, 3, 'This window is drawn by a Rust core in WebAssembly.', 34, 2);
    // Buttons go bottom-right; the toolkit places them. ~O~ is the hotkey:
    // Alt+O. The only button is the default (Enter) and the last (Escape).
    owl.buttons(w, ['~O~K', CmOk]);
  }

  /** Every button pressed arrives here. Return false to end the program. */
  onCommand(cmd) {
    return cmd !== CmOk;
  }
}
