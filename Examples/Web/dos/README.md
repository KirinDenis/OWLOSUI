# What the demo's DOS PC carries that this repository does not build

`owlfly3_v17.jsdos` is **OWL FLY III**, a flight game for DOS written in
8086 assembly, with its art and sound: a js-dos bundle, which is a zip with
a `dosbox.conf` in it. The demo does not boot the bundle as it is; it
unpacks the game onto its own PC's drive, as `C:\GAMES\OWLFLY3`, beside the
OWLOSUI examples, so every DOS setting in the demo applies to it too.

It is played over the network: DOSBox's IPX card, carried in a WebSocket to
a relay that hands every packet to the other machines in the same room. The
demo's DOS > Network monitor shows that traffic.

The game's source, its manual and the relay are in
[wire-city-2](https://github.com/KirinDenis/wire-city-2/tree/main/GAMES/OWLFLY3),
by the same author, MIT. The `_v17` is that build's number: a bundle with a
new number is a new file, so a browser never serves an old one from its cache.
