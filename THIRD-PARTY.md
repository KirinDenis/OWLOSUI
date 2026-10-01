# What in this repository is not this project's

Everything else is MIT, in [LICENSE](LICENSE).

| What | Where | Licence | From |
|---|---|---|---|
| js-dos 8.3.20: DOSBox compiled to WebAssembly, and its loader | [lib/js/jsdos/](lib/js/jsdos/NOTICE.md) | **GPL-2.0** (libzip inside it: BSD-3-Clause) | [caiiiycuk/js-dos](https://github.com/caiiiycuk/js-dos), [js-dos](https://github.com/js-dos) |
| coi-serviceworker 0.1.7: makes a static host's page cross-origin isolated, which DOSBox's threads need | [Examples/Web/JavaScript/05-Demo/coi-serviceworker.js](Examples/Web/JavaScript/05-Demo/coi-serviceworker.js) | MIT, Guido Zuidhof and contributors | [gzuidhof/coi-serviceworker](https://github.com/gzuidhof/coi-serviceworker) |
| CWSDPMI: the DPMI host the DOS programs start with | `lib/dos/CWSDPMI.EXE`, `Examples/DOS/Rust/CWSDPMI.EXE` | Charles W. Sandmann's own terms | [sandmann.dotster.com/cwsdpmi](http://sandmann.dotster.com/cwsdpmi/) |

**The GPL stays in its folder.** js-dos is used exactly as published - not
patched, not rebuilt - and the page talks to it through its published
interface. Its corresponding source is upstream; its licence text is
[lib/js/jsdos/COPYING-GPL-2.0.txt](lib/js/jsdos/COPYING-GPL-2.0.txt). The DOS
programs that run in it, and the toolkit around it, are separate works under
MIT.

**OWL FLY III**, the game on the demo's DOS PC
([Examples/Web/dos/owlfly3_v17.jsdos](Examples/Web/dos/README.md)), is by the
same author as this project, MIT, from
[wire-city-2](https://github.com/KirinDenis/wire-city-2/tree/main/GAMES/OWLFLY3).
