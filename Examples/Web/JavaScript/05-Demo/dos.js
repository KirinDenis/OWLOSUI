// The DOS menu: a real DOS PC in the page, in a window of the demo.
//
// One PC, put together here from this repository's own files when it is
// switched on (DISK, below):
//
//   C:\OWLOS        the toolkit for DOS: OWLOSRES, the same Rust core as
//                   this page's, built for DOS and resident behind INT 60h
//   C:\DEMO         the DOS examples - the commander, the demo in Pascal,
//                   C and assembler - with their sources
//   C:\GAMES        OWL FLY III, a networked flight game for DOS
//   A:, B:          floppies that are folders of this browser's storage -
//                   "DOS A Drive" and "DOS B Drive": what the page puts
//                   there DOS finds, what DOS saves there the page keeps
//                   (lib/js/dosbox/gates.js)
//
// What it starts into is a line in its AUTOEXEC: the commander, the demo,
// OWL FLY III, or the prompt. Settings (DOSBox's own: video card, CPU,
// sound, memory, network) are a dialog with a page for each, and they are
// written out as the dosbox.conf the machine boots with - which the dialog
// can show.
//
// The machine is ../../../../lib/js/dosbox/dosbox.js; this file is what the
// demo does with it.

import { Style } from '../../../../lib/js/owlosui.js';
import { DosBox, KEYS } from '../../../../lib/js/dosbox/dosbox.js';
import { unzip } from '../../../../lib/js/dosbox/unzip.js';
import { DriveGates } from '../../../../lib/js/dosbox/gates.js';
import { Monitors, MonCm } from './monitors.js';
import { log, describe } from './log.js';

/** The floppies, and the folders of this browser's storage they are. */
const GATES = [{ letter: 'A', folder: '/DOS A Drive/' }, { letter: 'B', folder: '/DOS B Drive/' }];

/** The repository, as the page's server shows it. */
const ROOT = new URL('../../../../', import.meta.url);

export const DosCm = {
  Commander: 350, Demo: 351, Prompt: 352, OwlFly: 353, Settings: 354,
  NetMonitor: 355, MachineMonitor: 356, Stop: 357, Keys: 358, Close: 359, Restart: 360,
};
const Cm = { SetOk: 370, SetCancel: 371, SetConf: 372, Page: 380 };

/** Drive C:, from the repository: [where on C:, where in the repository]. */
const DISK = [
  ['OWLOS/CWSDPMI.EXE', 'lib/dos/CWSDPMI.EXE'],
  ['OWLOS/OWLOSRES.COM', 'lib/dos/OWLOSRES.COM'],
  ['OWLOS/OWLOSRES.BIN', 'lib/dos/OWLOSRES.BIN'],
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
];
/** OWL FLY III is a js-dos bundle, unpacked onto C:\GAMES\OWLFLY3. */
const OWLFLY = 'Examples/Web/dos/owlfly3_v17.jsdos';

const crlf = s => s.replace(/\r?\n/g, '\r\n');

/** The batch files and the words on C: and A:, written here. */
const WRITTEN = {
  'C/OWL.BAT': `@echo off
rem Runs a program with the OWLOSUI toolkit behind it: CWSDPMI, which
rem gives the 32-bit core its memory, then OWLOSRES, the core itself,
rem which stays for exactly the one program it is given.
C:\\OWLOS\\CWSDPMI.EXE
C:\\OWLOS\\OWLOSRES.COM %1 %2 %3 %4
`,
  // /STEP: the commander may step out of memory to run a program - the
  // program's lines in RUNPROG.BAT - and is started again afterwards, in
  // the same folders. A DOS program then has all of DOS's memory, as from
  // the prompt; OWL FLY III needs it.
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

/** What the machine starts into: a line of its AUTOEXEC. */
const STARTS = {
  commander: { name: 'the commander', line: 'call C:\\COMMANDR.BAT' },
  demo: { name: 'the OWLOSUI demo', line: 'call C:\\DEMO.BAT' },
  owlfly: { name: 'OWL FLY III', line: 'call C:\\OWLFLY.BAT', network: true },
  prompt: { name: 'the DOS prompt', line: '' },
};

/** DOSBox's settings, as the dialog shows them; DEFAULTS is what a first visit gets. */
const DEFAULTS = {
  machine: 'svga_s3', memsize: 16, picture: 'sharp',
  core: 'auto', cputype: 'auto', cycles: 'auto', fixed: 20000,
  sbtype: 'sb16', gus: false, pcspeaker: true, rate: 44100,
  ems: true, umb: true,         // XMS is always on: see dosboxConf
  awake: true,
  start: 'commander',
  ipx: true, always: true, relay: 'wss://view.owlos.sk', room: 'owlfly3',
};

/**
 * The memory sizes on offer, in MB. Nothing under 16: at 4 MB the toolkit
 * core's DPMI memory does not fit and every OWLOSUI program fails with
 * "error 8"; at 1 MB there is no extended memory at all, and DOS-extended
 * programs (DOS/16M, DOS4GW) stop with "not enough extended memory".
 */
const MEMORY = [16, 32, 63];

function loadSettings() {
  let s;
  try { s = { ...DEFAULTS, ...JSON.parse(localStorage.getItem('owlosui.dosbox') ?? '{}') }; } catch { s = { ...DEFAULTS }; }
  // A size saved before the small ones were withdrawn: lift it.
  if (!MEMORY.includes(s.memsize)) s.memsize = DEFAULTS.memsize;
  return s;
}
function saveSettings(s) {
  try { localStorage.setItem('owlosui.dosbox', JSON.stringify(s)); } catch { /* a private window: for this visit only */ }
}

/**
 * The dosbox.conf a machine boots with: the settings, and what to start.
 * XMS is always on, and not a setting: without it CWSDPMI takes the
 * toolkit core's memory from DOS's own 640K, DOS has none left to load
 * the program, and every OWLOSUI program fails with "error 8".
 */
export function dosboxConf(s, start, line = STARTS[start]?.line ?? '') {
  const yes = b => (b ? 'true' : 'false');
  return crlf(`[sdl]
autolock=false
[dosbox]
machine=${s.machine}
memsize=${s.memsize}
[cpu]
core=${s.core}
cputype=${s.cputype}
cycles=${s.cycles === 'fixed' ? `fixed ${s.fixed}` : s.cycles}
[mixer]
rate=${s.rate}
[sblaster]
sbtype=${s.sbtype}
[gus]
gus=${yes(s.gus)}
[speaker]
pcspeaker=${yes(s.pcspeaker)}
[dos]
xms=true
ems=${yes(s.ems)}
umb=${yes(s.umb)}
[ipx]
ipx=${yes(s.ipx)}
[autoexec]
@echo off
mount c C
mount a A -t floppy
mount b B -t floppy
path C:\\;Z:\\
c:
cls
${line}
type C:\\README.TXT
`);
}

/**
 * The settings dialog's pages. Each builds its controls in the dialog and
 * reads them back into the draft; a page button rebuilds the dialog on
 * another page, the draft carried across.
 */
const PAGES = [
  {
    name: 'Machine',
    build(o, d, s) {
      o.staticText(d.win, 2, 3, 'Video card:', 20);
      d.parts.machine = o.cluster(d.win, 2, 4, 28, ['S3 Trio64 SVGA', 'VGA', 'EGA', 'CGA', 'Hercules', 'Tandy'],
        { single: true, on: ['svga_s3', 'vgaonly', 'ega', 'cga', 'hercules', 'tandy'].indexOf(s.machine) });
      o.staticText(d.win, 34, 3, 'Memory:', 20);
      d.parts.memsize = o.cluster(d.win, 34, 4, 20, MEMORY.map(m => `${m} MB`),
        { single: true, on: MEMORY.indexOf(s.memsize) });
      o.staticText(d.win, 34, 10, 'Picture:', 20);
      d.parts.picture = o.cluster(d.win, 34, 11, 28, ['sharp: whole pixels', 'a monitor: 4:3, smoothed'],
        { single: true, on: ['sharp', 'monitor'].indexOf(s.picture) });
      o.staticText(d.win, 2, 11, 'The card decides what a program may draw: OWL FLY III asks the machine which modes it has.', 30, 4);
    },
    read(o, d, s) {
      s.machine = ['svga_s3', 'vgaonly', 'ega', 'cga', 'hercules', 'tandy'][radio(o, d.parts.machine)];
      s.memsize = MEMORY[radio(o, d.parts.memsize)];
      s.picture = ['sharp', 'monitor'][radio(o, d.parts.picture)];
    },
  },
  {
    name: 'CPU',
    build(o, d, s) {
      o.staticText(d.win, 2, 3, 'Core:', 20);
      d.parts.core = o.cluster(d.win, 2, 4, 24, ['auto', 'dynamic: fast', 'normal: exact', 'simple'],
        { single: true, on: ['auto', 'dynamic', 'normal', 'simple'].indexOf(s.core) });
      o.staticText(d.win, 30, 3, 'Processor:', 20);
      d.parts.cputype = o.cluster(d.win, 30, 4, 20, ['auto', '386', '486', 'Pentium'],
        { single: true, on: ['auto', '386', '486_slow', 'pentium_slow'].indexOf(s.cputype) });
      o.staticText(d.win, 2, 9, 'Speed:', 20);
      d.parts.cycles = o.cluster(d.win, 2, 10, 24, ['auto', 'as fast as it goes', 'fixed, cycles/ms:'],
        { single: true, on: ['auto', 'max', 'fixed'].indexOf(s.cycles) });
      d.parts.fixed = o.input(d.win, 28, 12, 10, '', String(s.fixed), 7);
    },
    read(o, d, s) {
      s.core = ['auto', 'dynamic', 'normal', 'simple'][radio(o, d.parts.core)];
      s.cputype = ['auto', '386', '486_slow', 'pentium_slow'][radio(o, d.parts.cputype)];
      s.cycles = ['auto', 'max', 'fixed'][radio(o, d.parts.cycles)];
      s.fixed = Math.max(100, Math.min(1000000, parseInt(o.getText(d.parts.fixed), 10) || DEFAULTS.fixed));
    },
  },
  {
    name: 'Sound',
    build(o, d, s) {
      o.staticText(d.win, 2, 3, 'Sound card:', 20);
      d.parts.sbtype = o.cluster(d.win, 2, 4, 26, ['Sound Blaster 16', 'Sound Blaster Pro 2', 'Sound Blaster 2.0', 'none'],
        { single: true, on: ['sb16', 'sbpro2', 'sb2', 'none'].indexOf(s.sbtype) });
      o.staticText(d.win, 32, 3, 'Also:', 20);
      d.parts.also = o.cluster(d.win, 32, 4, 26, ['Gravis UltraSound', 'PC speaker'], { on: [s.gus, s.pcspeaker] });
      o.staticText(d.win, 32, 7, 'Mixer rate:', 20);
      d.parts.rate = o.cluster(d.win, 32, 8, 20, ['44100 Hz', '22050 Hz', '11025 Hz'],
        { single: true, on: [44100, 22050, 11025].indexOf(s.rate) });
    },
    read(o, d, s) {
      s.sbtype = ['sb16', 'sbpro2', 'sb2', 'none'][radio(o, d.parts.sbtype)];
      [s.gus, s.pcspeaker] = o.clusterState(d.parts.also).on;
      s.rate = [44100, 22050, 11025][radio(o, d.parts.rate)];
    },
  },
  {
    name: 'DOS',
    build(o, d, s) {
      o.staticText(d.win, 2, 3, 'Memory DOS offers:', 26);
      // No XMS here: it is always on - the toolkit's core lives in it.
      d.parts.mem = o.cluster(d.win, 2, 4, 30, ['EMS', 'Upper memory blocks'], { on: [s.ems, s.umb] });
      o.staticText(d.win, 34, 3, 'At start, run:', 26);
      d.parts.start = o.cluster(d.win, 34, 4, 26, ['the commander', 'the OWLOSUI demo', 'OWL FLY III', 'the DOS prompt'],
        { single: true, on: ['commander', 'demo', 'owlfly', 'prompt'].indexOf(s.start) });
      o.staticText(d.win, 2, 10, 'DOS > Switch on runs this; the other DOS menu items say what to run themselves.', 60, 2);
      d.parts.awake = o.cluster(d.win, 2, 13, 60, ['Keep running in a background tab - [awake] on its window'], { on: [s.awake] });
    },
    read(o, d, s) {
      [s.ems, s.umb] = o.clusterState(d.parts.mem).on;
      s.start = ['commander', 'demo', 'owlfly', 'prompt'][radio(o, d.parts.start)];
      [s.awake] = o.clusterState(d.parts.awake).on;
    },
  },
  {
    name: 'Network',
    build(o, d, s) {
      d.parts.net = o.cluster(d.win, 2, 3, 60, ['IPX network card', 'Connect every program, not only OWL FLY III'], { on: [s.ipx, s.always] });
      d.parts.relay = o.input(d.win, 2, 7, 58, 'Relay', s.relay, 200);
      d.parts.room = o.input(d.win, 2, 9, 30, 'Room', s.room, 40);
      o.staticText(d.win, 2, 11, 'A DOS network card in a browser: each IPX packet goes in a WebSocket to the relay, ' +
        'which hands it to every machine in the same room. Players of OWL FLY III meet in room owlfly3.', 60, 3);
    },
    read(o, d, s) {
      [s.ipx, s.always] = o.clusterState(d.parts.net).on;
      s.relay = o.getText(d.parts.relay).trim() || DEFAULTS.relay;
      s.room = o.getText(d.parts.room).trim().replace(/[^\w-]/g, '') || DEFAULTS.room;
    },
  },
];

const radio = (o, id) => Math.max(0, o.clusterState(id).on.indexOf(true));

export class DosTool {
  static CmFirst = 350;
  static CmLast = 399;

  /**
   * commander: the page's commander, told when DOS changes a gate folder.
   * storage: this browser's storage, where the gate folders are. open: how
   * the demo puts a text in an editor (for "Show dosbox.conf").
   */
  constructor(owl, { commander, storage, open }) {
    this.owl = owl;
    this.box = new DosBox(owl);
    this.storage = storage;
    // The floppies are folders of this browser's storage, there to see.
    this.gates = new DriveGates(this.box, storage, GATES);
    this.gates.pushed = () => { log.info('files', 'the page wrote to a floppy: DOS reads it again'); this.floppyChanged(); };
    this.gates.activity = letter => this.light(letter);
    this.commander = commander;
    this.open = open;
    this.settings = loadSettings();
    this.win = 0;
    this.started = null;      // which of STARTS runs
    this.dialog = null;
    this.disk = null;         // drive C:'s files, fetched once
    this.network = { state: 'off', url: '' };
    this.lit = new Set();        // the drives whose light is on
    this.lightsOff = {};
    let keys = null;
    this.box.on((what, box, detail) => {
      if (what === 'keys') {
        this.retitle();
        // Said when it changes hands, not on every focus event.
        if (box.hasKeys() !== keys) { keys = box.hasKeys(); log.info('dos', keys ? 'the keyboard is in DOS' : 'the keyboard is back on the page'); }
      }
      if (what === 'stopped') { log.info('dos', 'switched off'); if (this.win) this.retitle(); }
      if (what === 'frame') log.info('dos', `picture ${box.frameSize.w}x${box.frameSize.h}`);
      if (what === 'stdout') log.info('dos out', detail.replace(/\s+$/, ''));
      // DOSBox writes its own [LOG_...] lines to stderr and js-dos hands
      // them on as errors; they are its diary, not its troubles, and shown
      // red they would hide the real ones.
      if (what === 'message') {
        const level = /^\s*\[LOG_/.test(detail.text) ? 'info' : detail.type === 'error' ? 'error' : detail.type === 'warn' ? 'warn' : 'info';
        log.write('dos out', detail.text, level);
      }
    });
    this.monitors = new Monitors(owl, this);
    if (typeof setInterval !== 'undefined') setInterval(() => this.watchFloppy(), 1500);
    // The folders are made on the first visit, before anyone looks for them.
    if (typeof navigator !== 'undefined' && navigator.storage?.getDirectory) this.gates.ensure().catch(() => {});
  }

  handles(cmd) { return cmd >= DosTool.CmFirst && cmd <= DosTool.CmLast; }
  owns(id) { return id !== 0 && (id === this.win || id === this.dialog?.win || this.monitors.owns(id)); }
  poll() {}

  /** The DOS menu's items, for the demo's menu bar. */
  menu() {
    return [
      { label: 'Switch ~o~n', cmd: DosCm.Restart, hint: 'A DOS PC in a window: DOSBox, compiled to WebAssembly, starting what Settings say' },
      { label: '~C~ommander on DOS', cmd: DosCm.Commander, hint: "The file manager, in Pascal, on the same Rust core as this page - built for DOS" },
      { label: 'OWLOSUI ~d~emo on DOS', cmd: DosCm.Demo, hint: 'This demo, written again in Pascal, on DOS' },
      { label: 'Play OWL ~F~LY III', cmd: DosCm.OwlFly, hint: 'A DOS flight game on the network: every player in the same room shares the sky' },
      { label: 'DOS ~p~rompt', cmd: DosCm.Prompt, hint: 'Just DOS: type COMMANDR, DEMO or OWLFLY' },
      { label: '', separator: true },
      { label: '~S~ettings...', cmd: DosCm.Settings, hint: "DOSBox's own settings - video card, CPU, sound, memory, network - and the dosbox.conf they make" },
      { label: '~N~etwork monitor', cmd: DosCm.NetMonitor, hint: 'What the DOS machine says on the network: bytes, a graph, every packet' },
      { label: '~M~achine monitor', cmd: DosCm.MachineMonitor, hint: 'How fast the emulated PC runs, how busy it is, what is on its disks' },
      { label: 'S~t~op', cmd: DosCm.Stop, hint: 'Switch the DOS PC off' },
    ];
  }

  onCommand(cmd) {
    switch (cmd) {
      case DosCm.Commander: return this.run('commander');
      case DosCm.Demo: return this.run('demo');
      case DosCm.OwlFly: return this.run('owlfly');
      case DosCm.Prompt: return this.run('prompt');
      case DosCm.Restart: return this.run(this.started ?? this.settings.start);
      case DosCm.Stop: return this.stop();
      case DosCm.Close: return this.closeWindow(this.win);
      case DosCm.Keys: this.box.grab(); return null;
      case DosCm.Settings: return this.showSettings(0);
      case DosCm.NetMonitor: return this.monitors.showNet();
      case DosCm.MachineMonitor: return this.monitors.showMachine();
      case MonCm.NetClose: case MonCm.MachineClose: return this.monitors.onCommand(cmd);
      case Cm.SetOk: return this.settingsOk();
      case Cm.SetCancel: return this.closeDialog();
      case Cm.SetConf: return this.showConf();
    }
    if (cmd >= Cm.Page && cmd < Cm.Page + PAGES.length) this.showSettings(cmd - Cm.Page);
    return null;
  }

  // ------------------------------------------------------------ the window

  /** Where the window goes when nothing says: centred, its inside as near 4:3 as the screen allows. */
  defaultRect() {
    const W = this.owl.width, H = this.owl.height;
    const cw = this.owl.screen?.cellW ?? 10, ch = this.owl.screen?.cellH ?? 22;
    let iw, ih;
    if (this.settings.picture === 'sharp') {
      // The window round the picture: DOS's 640 by 400 at the largest
      // whole scale the screen has room for.
      ({ w: iw, h: ih } = DosBox.insideFor(this.owl, W - 4, H - 6));
    } else {
      ih = H - 6;
      iw = Math.round((ih * ch * 4) / 3 / cw);
      if (iw > W - 4) { iw = W - 4; ih = Math.round((iw * cw * 3) / 4 / ch); }
    }
    return { x: Math.floor((W - iw - 2) / 2), y: 1 + Math.floor((H - 2 - ih - 2) / 2), w: iw + 2, h: ih + 2 };
  }

  openWindow(rect = this.defaultRect()) {
    const owl = this.owl;
    if (this.win) owl.close(this.win);
    this.win = owl.window('DOS', rect.w, rect.h, { x: rect.x, y: rect.y, closeCmd: DosCm.Close });
    // What shows when the picture does not: while something is over the
    // window, or before the machine is on.
    owl.staticText(this.win, 2, 1, 'The DOS PC is switching on. Its picture is laid over this window, and taken away ' +
      'while a menu or another window is over it - DOS keeps running. Bring this window to the front to see it again.', rect.w - 6, 6);
    // Right Ctrl, the one key worth saying; a click on the picture is how
    // the keyboard goes to DOS.
    owl.windowStatus(this.win, { label: '~Right Ctrl~ Keys back', cmd: 0 });
    this.showLights();
    owl.windowMenu(this.win, {
      label: '~M~achine', items: [
        { label: '~K~eyboard to DOS', cmd: DosCm.Keys, hint: 'Or click the DOS picture; Right Ctrl gives it back' },
        { label: '~R~estart', cmd: DosCm.Restart, hint: 'Switch off and on again, with the settings as they are now' },
        { label: '~S~ettings...', cmd: DosCm.Settings },
        { label: '~N~etwork monitor', cmd: DosCm.NetMonitor },
        { label: '~M~achine monitor', cmd: DosCm.MachineMonitor },
      ],
    });
    owl.activate(this.win);
  }

  retitle() {
    if (!this.win) return;
    // Short: a title longer than its window has room for is not drawn at
    // all. The status line says the rest - Enter, Right Ctrl.
    const name = this.program?.name ?? STARTS[this.started]?.name ?? 'DOS';
    const state = !this.box.running ? ' (off)' : this.box.hasKeys() ? ' - keys in DOS' : '';
    this.owl.setText(this.win, `DOS: ${name}${state}`);
    // [awake], beside the title as [modal] is, while it would keep running hidden.
    this.owl.windowTag(this.win, this.box.running && this.box.keepRunning ? 'awake' : '');
    this.owl.refresh?.();
  }

  /**
   * Switch the PC on (again) and start `start` - a key of STARTS. rect:
   * where its window goes, if not centred.
   */
  async run(start, { rect = null, grab = true, program = null } = {}) {
    if (typeof document === 'undefined') return;
    this.started = start;
    this.program = program;
    this.cSeen = null;          // a new disk C: is not DOS writing to it
    if (!this.win || rect) this.openWindow(rect ?? undefined);
    else this.owl.activate(this.win);
    this.retitle();
    const s = this.settings;
    log.info('dos', `switching on to run ${program?.name ?? STARTS[start].name}: ${s.machine}, ${s.memsize} MB, ` +
      `cycles ${s.cycles === 'fixed' ? `fixed ${s.fixed}` : s.cycles}, ${s.awake ? 'awake' : 'pauses when hidden'}`);
    if (program) log.info('dos', `its lines: ${program.line.replace(/\n/g, ' | ')}`);
    try {
      // What DOS saved on its floppies in its last moments goes into the
      // folders before it is switched off; the folders are then the disks
      // of the PC switching on.
      await this.gates.pull().catch(() => {});
      const files = [...(await this.files()), ...(await this.gates.files())];
      await this.box.start(this.win, {
        conf: dosboxConf(this.settings, start, program?.line),
        files,
        what: program?.name ?? STARTS[start].name,
        // Awake: it keeps running in a background tab, as a PC does when
        // you look away from it. A browser pauses a hidden page's emulator
        // otherwise - and a game played in step over the network, Duke
        // Nukem 3D, waits for every player, so one hidden tab stops it for
        // all of them. Settings > DOS can turn it off, to spare the battery.
        keepRunning: this.settings.awake,
        picture: this.settings.picture,
      });
      await this.gates.started();
      log.info('dos', `on: ${files.length} files on its disks`);
      if (STARTS[start].network || this.settings.always) this.connect();
      else this.network = { state: 'off', url: '' };
      if (grab) this.box.grab();
    } catch (e) {
      log.error('dos', `did not start: ${describe(e)}`);
      this.owl.messageBox('DOS', `The DOS PC did not start: ${e.message}`, { label: '~O~K', cmd: 0, default: true });
    }
    this.retitle();
  }

  /**
   * Enter on a program in the page's commander: it runs on this PC. One in
   * a gate folder - DOS A Drive, DOS B Drive - is already on a floppy and
   * runs from there. One from anywhere else is copied into DOS A Drive\RUN
   * first, with the files beside it - a program finds its data in its own
   * folder - and stays there, as the floppy does. A toolkit program - one
   * that asks for OWLOSRES, as each of them says it needs - runs with the
   * toolkit behind it. When it ends, the commander.
   */
  async runProgram(source, dir, name) {
    const BS = '\\';
    let path = `${dir}${name}`;
    if (!(source === this.storage && this.gates.gateOf(path))) {
      // RUN holds the last program run this way and its files, nothing older.
      const run = `${GATES[0].folder}RUN/`;
      await this.storage.remove(run).catch(() => {});
      await this.storage.mkdir(run);
      let room = 1400 * 1024;
      const own = await source.readBytes(path);
      await this.storage.write(`${run}${name.toUpperCase()}`, own);
      room -= own.length;
      const beside = (await source.list(dir)).filter(e => !e.dir && e.name !== name);
      for (const e of beside.sort((a, b) => a.size - b.size)) {
        if (e.size > room) continue;
        await this.storage.write(`${run}${e.name.toUpperCase()}`, await source.readBytes(`${dir}${e.name}`));
        room -= e.size;
      }
      path = `${run}${name.toUpperCase()}`;
    }
    const at = this.gates.dosPath(path);              // A:\RUN\DEMO.COM
    const folder = at.slice(0, at.lastIndexOf(BS)) || at.slice(0, 2);
    const upper = name.toUpperCase();
    const toolkit = new TextDecoder('latin1').decode(await this.storage.readBytes(path)).includes('OWLOSRES');
    const run = upper.endsWith('.BAT') ? `call ${upper}` : toolkit ? `call C:${BS}OWL.BAT ${upper}` : upper;
    log.info('dos', `run ${at} from the page's commander${toolkit ? ', with the toolkit behind it' : ''}`);
    const line = [folder.slice(0, 2), `cd ${folder.slice(2) || BS}`, run, 'C:', `cd ${BS}`, `call C:${BS}COMMANDR.BAT`].join('\n');
    return this.run('commander', { program: { name: upper, line } });
  }

  /** Drive C:, fetched from the repository once and kept. */
  async files() {
    if (!this.disk) {
      const get = async path => {
        const r = await fetch(new URL(path, ROOT));
        if (!r.ok) throw new Error(`${path}: ${r.status} ${r.statusText}`);
        return new Uint8Array(await r.arrayBuffer());
      };
      const disk = await Promise.all(DISK.map(async ([to, from]) => ({ path: `C/${to}`, contents: await get(from) })));
      for (const f of await unzip(await get(OWLFLY))) {
        if (f.path.startsWith('.jsdos/') || f.path.toLowerCase() === 'dosbox.conf') continue;
        disk.push({ path: `C/GAMES/OWLFLY3/${f.path}`, contents: f.contents });
      }
      this.disk = disk;
    }
    const written = Object.entries(WRITTEN).map(([path, text]) => ({ path, contents: new TextEncoder().encode(crlf(text)) }));
    return [...this.disk.map(f => ({ path: f.path, contents: f.contents.slice() })), ...written];
  }

  /** On the relay: the machine's IPX card, cabled through a WebSocket. */
  async connect() {
    const url = `${this.settings.relay.replace(/\/+$/, '')}/ipx/${this.settings.room}`;
    this.network = { state: 'connecting', url };
    log.info('net', `connecting the IPX card to ${url}`);
    try {
      await this.box.ci.networkConnect(0, url);
      this.network = { state: 'connected', url };
      log.info('net', 'connected');
    } catch (e) {
      this.network = { state: `not connected: ${e?.message ?? 'no answer'}`, url };
      log.warn('net', this.network.state);
    }
    this.owl.refresh?.();
  }

  async stop() {
    await this.box.stop();
    this.network = { state: 'off', url: '' };
    this.retitle();
  }

  /** Alt+F3 or the close box on the DOS window: the PC is switched off with it. */
  async closeWindow(id) {
    if (id === this.dialog?.win) return this.closeDialog();
    if (this.monitors.owns(id)) return this.monitors.closeWindow(id);
    if (id !== this.win || !id) return;
    this.owl.close(this.win);
    this.win = 0;
    this.started = null;
    await this.box.stop();
    this.owl.refresh?.();
  }

  // ------------------------------------------------------------ the floppies

  /**
   * The page wrote to a floppy. DOS's commander shows a folder as it
   * read it, so it is asked to read again - Ctrl+R, pressed for the person.
   */
  floppyChanged() {
    clearTimeout(this.rereadSoon);
    this.rereadSoon = setTimeout(() => {
      if (this.box.running && this.started === 'commander') this.box.press(KEYS.ctrl, KEYS.r);
    }, 250);
  }

  /**
   * What DOS wrote to its floppies, into the folders - and the page's
   * commander reads them again. And whether DOS wrote to its hard disk,
   * for C:'s light: the emulator tells the page what its files are and how
   * big, so a change is seen; what DOS only reads is not.
   */
  async watchFloppy() {
    if (!this.box.running || this.pulling) return;
    this.pulling = true;
    try {
      if (await this.gates.pull()) {
        log.info('files', 'DOS wrote to a floppy: the folder in this browser has it now');
        this.commander?.refreshSource?.(this.storage);
      }
      const tree = await this.box.ci.fsTree();
      const c = JSON.stringify((tree.nodes ?? []).find(n => n.name === 'C'));
      if (this.cSeen && c !== this.cSeen) this.light('C');
      this.cSeen = c;
    } catch { /* switching off */ }
    this.pulling = false;
  }

  /**
   * A drive's light, at the right of the status line while the DOS window
   * is in front: [A: B: C:], a letter green while files go to or from it.
   */
  light(letter) {
    this.lit.add(letter);
    this.showLights();
    clearTimeout(this.lightsOff[letter]);
    this.lightsOff[letter] = setTimeout(() => { this.lit.delete(letter); this.showLights(); }, 900);
  }

  showLights() {
    if (!this.win) return;
    const drives = ['A', 'B', 'C'].map(l => (this.lit.has(l) ? `~${l}:~` : `${l}:`)).join(' ');
    this.owl.windowIndicator(this.win, `[${drives}]`);
    this.owl.refresh?.();
  }

  // ------------------------------------------------------------ settings

  /** DOS > Settings: one page of the dialog; the page buttons along the top change it. */
  showSettings(page) {
    const owl = this.owl;
    const draft = this.dialog?.draft ?? { ...this.settings };
    if (this.dialog) {
      PAGES[this.dialog.page].read(owl, this.dialog, draft);
      owl.close(this.dialog.win);
    }
    const win = owl.window('DOSBox settings', 66, 20, { style: Style.ModalDialog, closeCmd: Cm.SetCancel });
    // The pages, as a row of buttons; the one shown is marked.
    owl.buttonRow(win, 1, 0, PAGES.map((p, i) => ({ label: i === page ? `[${p.name}]` : p.name, cmd: Cm.Page + i, style: i === page ? 'accent' : 'normal' })), { selectable: false });
    this.dialog = { win, page, draft, parts: {} };
    PAGES[page].build(owl, this.dialog, draft);
    owl.buttons(win,
      { label: '~O~K', cmd: Cm.SetOk, default: true },
      { label: 'dosbox.~c~onf', cmd: Cm.SetConf },
      { label: 'Cancel', cmd: Cm.SetCancel, cancel: true });
  }

  closeDialog() {
    if (this.dialog) this.owl.close(this.dialog.win);
    this.dialog = null;
  }

  /** OK: the settings kept, and a running PC switched on again with them. */
  settingsOk() {
    const d = this.dialog;
    PAGES[d.page].read(this.owl, d, d.draft);
    const changed = Object.keys(d.draft).filter(k => d.draft[k] !== this.settings[k]).map(k => `${k} ${this.settings[k]} -> ${d.draft[k]}`);
    log.info('dos', `settings: ${changed.length ? changed.join(', ') : 'nothing changed'}`);
    this.settings = d.draft;
    saveSettings(this.settings);
    this.closeDialog();
    if (this.box.running) this.run(this.started ?? this.settings.start);
  }

  /** The dosbox.conf the settings in the dialog make, in an editor: DOSBox's own words for them. */
  showConf() {
    const d = this.dialog;
    PAGES[d.page].read(this.owl, d, d.draft);
    const text = dosboxConf(d.draft, this.started ?? d.draft.start).replace(/\r\n/g, '\n');
    this.closeDialog();
    this.open('dosbox.conf', text, null, null, { readOnly: true });
  }
}
