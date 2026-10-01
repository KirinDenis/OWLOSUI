// What to see: the window a first-time visitor meets, in the middle of the
// screen, so nobody has to guess where to click. A list of what the demo
// has, a few words on each as the cursor moves, and Enter to be shown it.
// Help > What to see brings it back.

import { Style } from '../../../../lib/js/owlosui.js';

const Cm = { Go: 420, Close: 421 };

export class Welcome {
  static CmFirst = 420;
  static CmLast = 429;

  /** choices: [{ label, about, go() }], in the order the list shows them. */
  constructor(owl, choices) {
    this.owl = owl;
    this.choices = choices;
    this.win = 0;
  }

  handles(cmd) { return cmd >= Welcome.CmFirst && cmd <= Welcome.CmLast; }
  owns(id) { return id !== 0 && id === this.win; }

  show() {
    const owl = this.owl;
    if (this.win) { owl.activate(this.win); return; }
    const w = Math.min(76, owl.width), h = Math.min(21, owl.height - 2);
    this.win = owl.window('What to see', w, h, { style: Style.Dialog, closeCmd: Cm.Close });
    owl.staticText(this.win, 2, 1, 'OWLOSUI is a text-mode toolkit with one core, written in Rust. Every window on this ' +
      'page is drawn by that core, compiled to WebAssembly - and the same core runs on DOS, in a DOS PC this page can switch on.', w - 6, 3);
    this.list = owl.list(this.win, 2, 5, 34, this.choices.length, this.choices.map(c => c.label));
    this.about = owl.staticText(this.win, 39, 5, '', w - 43, h - 10);
    owl.staticText(this.win, 2, h - 6, 'Enter shows it; Help > What to see brings this window back.', w - 6, 1);
    owl.buttons(this.win, { label: '~S~how me', cmd: Cm.Go, default: true }, { label: 'Close', cmd: Cm.Close, cancel: true });
    this.shown = -1;
    this.poll();
  }

  /** The words beside the list follow its cursor. */
  poll() {
    if (!this.win) return;
    const i = this.owl.current(this.list);
    if (i === this.shown) return;
    this.shown = i;
    this.owl.setText(this.about, this.choices[i]?.about ?? '');
  }

  onCommand(cmd) {
    if (cmd === Cm.Close) return this.close();
    if (cmd === Cm.Go) {
      const choice = this.choices[this.owl.current(this.list)];
      this.close();
      return choice?.go();
    }
    return null;
  }

  close() {
    if (this.win) this.owl.close(this.win);
    this.win = 0;
  }

  closeWindow() { this.close(); }
}
