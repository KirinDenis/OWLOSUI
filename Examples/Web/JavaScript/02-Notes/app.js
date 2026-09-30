// Web, JavaScript, step 2 of 5 - an editor that keeps its text, finds and replaces, and asks before leaving.
// Before: 01-HelloWorld. Next: 03-Calculator, a keypad of real buttons.
//
// Examples/Desktop/CSharp/02-Notes in the browser. The one difference is
// where the text is kept: a browser page has no files, so it goes into
// localStorage under a name - "NOTES.TXT" - and is there again next time.
//
// Two things this shows that HelloWorld does not:
//
//  * The program never sees a keystroke of the editor. Typing, the caret,
//    selection, undo, the clipboard are all the core's; the program looks
//    at the text only when it has to - on Save, and to know whether it
//    changed.
//
//  * A question before losing work. The close box and Exit send CmExit
//    instead of closing, and the program decides: nothing changed, go;
//    otherwise ask, with Stay as the default so Enter and Escape both keep
//    you here.

import { Style } from '../../../../lib/js/owlosui.js';

export const CmSave = 1, CmExit = 2, CmLeave = 3, CmStay = 4;
export const CmFind = 5, CmReplace = 6, CmFindOk = 8, CmReplaceOne = 9, CmReplaceAll = 10, CmSearchCancel = 11;

export class App {
  /**
   * `storage` is anything with getItem and setItem: localStorage in a page,
   * a Map-like stand-in in a test.
   */
  constructor(owl, storage = globalThis.localStorage, name = 'NOTES.TXT') {
    this.owl = owl;
    this.storage = storage;
    this.name = name;
    this.saved = storage?.getItem('owlosui:' + name) ?? '';
    this.box = 0;
    this.search = 0;
    this.lastPattern = '';
    this.lastWith = '';

    // A document window almost as big as the desktop. closeCmd: the close
    // box sends CmExit instead of closing, so the program can ask first.
    this.window = owl.window(name, owl.width - 4, owl.height - 2, { closeCmd: CmExit });
    // The editor fills the window and has the focus: typing starts at once.
    this.editor = owl.text(this.window, this.saved);
    // Alt+S, Alt+F, Alt+R, Alt+X from anywhere. Escape is the last, Exit.
    owl.buttons(this.window, ['~S~ave', CmSave], ['~F~ind', CmFind], ['~R~eplace', CmReplace], ['E~x~it', CmExit]);
  }

  get modified() {
    return this.owl.getText(this.editor) !== this.saved;
  }

  /** Find, or Replace: fields, the two options every search has, buttons. Modal. */
  showSearch(replace) {
    const owl = this.owl;
    this.closeSearch();
    this.search = owl.window(replace ? 'Replace' : 'Find', 50, replace ? 12 : 10, { style: Style.ModalDialog, closeCmd: CmSearchCancel });
    this.findField = owl.input(this.search, 2, 1, 44, 'Find:', this.lastPattern);
    if (this.lastPattern) owl.setHistory(this.findField, this.lastPattern);
    this.withField = replace ? owl.input(this.search, 2, 3, 44, 'Replace with:', this.lastWith) : 0;
    this.options = owl.cluster(this.search, 2, replace ? 5 : 3, 30, ['~C~ase sensitive', '~W~hole words only']);
    if (replace) {
      owl.buttons(this.search, { label: '~R~eplace', cmd: CmReplaceOne, default: true }, ['Replace ~a~ll', CmReplaceAll],
        { label: '~C~ancel', cmd: CmSearchCancel, cancel: true });
    } else {
      owl.buttons(this.search, { label: '~O~K', cmd: CmFindOk, default: true }, { label: '~C~ancel', cmd: CmSearchCancel, cancel: true });
    }
  }

  /** What the dialog says, kept for next time. */
  takeSearch() {
    this.lastPattern = this.owl.getText(this.findField);
    if (this.withField) this.lastWith = this.owl.getText(this.withField);
    const { on } = this.owl.clusterState(this.options);
    this.flags = { caseSensitive: on[0], wholeWord: on[1] };
  }

  closeSearch() {
    if (this.search) this.owl.close(this.search);
    this.search = 0;
    this.withField = 0;
  }

  tell(title, text) {
    this.box = this.owl.messageBox(title, text, ['~O~K', CmStay]);
  }

  onCommand(cmd) {
    const owl = this.owl;
    switch (cmd) {
      case CmSave:
        // The only time the program reads the text.
        this.saved = owl.getText(this.editor);
        this.storage?.setItem('owlosui:' + this.name, this.saved);
        return true;
      case CmExit:
        if (!this.modified) return false;
        // Stay is the default and the last: Enter and Escape both keep you
        // here. Leaving has to be chosen on purpose - Alt+L or a click.
        this.box = owl.messageBox('Confirm', `${this.name} has been changed. Leave without saving?`,
          ['~L~eave', CmLeave], { label: '~S~tay', cmd: CmStay, default: true });
        return true;
      case CmLeave:
        return false;
      case CmStay:
        owl.close(this.box);
        this.box = 0;
        return true;
      case CmFind:
        this.showSearch(false);
        return true;
      case CmReplace:
        this.showSearch(true);
        return true;
      case CmFindOk:
        this.takeSearch();
        this.closeSearch();
        if (this.lastPattern && !owl.find(this.editor, this.lastPattern, this.flags)) this.tell('Find', `"${this.lastPattern}" not found.`);
        return true;
      case CmReplaceOne: {
        // The dialog stays up: each Replace does one and finds the next.
        this.takeSearch();
        const { found } = owl.replace(this.editor, this.lastPattern, this.lastWith, this.flags);
        if (!found) this.closeSearch();
        return true;
      }
      case CmReplaceAll: {
        this.takeSearch();
        this.closeSearch();
        const n = owl.replaceAll(this.editor, this.lastPattern, this.lastWith, this.flags);
        this.tell('Replace', `${n} replaced.`);
        return true;
      }
      case CmSearchCancel:
        this.closeSearch();
        return true;
      default:
        return true;
    }
  }
}
