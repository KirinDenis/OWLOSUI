// The browser backend: cells in, keys and clicks out.
//
// This file knows three things: how to load the module, how to draw a
// grid of cells on a canvas, and how to turn the browser's events into
// the numbers the module expects. It does not know what a window is,
// what a menu is or what any key means - that is all inside the module,
// the same Rust core that draws the terminal version.
//
// The frame is three bytes a cell: glyph low, glyph high, attribute. The
// glyph is an index: below 256 it is a code page 437 character (the IBM
// PC's, so the frames and shades come out as they did on a DOS card);
// above it is the character's own code point, which is how the core's
// growing font works. The attribute is the IBM byte: foreground in the
// low nibble, background in the high one, sixteen colours.

const Owlosui = (() => {
  // The sixteen colours of the IBM CGA, as every emulator shows them.
  const PALETTE = [
    '#000000', '#0000aa', '#00aa00', '#00aaaa', '#aa0000', '#aa00aa', '#aa5500', '#aaaaaa',
    '#555555', '#5555ff', '#55ff55', '#55ffff', '#ff5555', '#ff55ff', '#ffff55', '#ffffff',
  ];

  // Code page 437, 0..255, as code points. The control range holds the
  // pictures the IBM PC put there: the faces, the arrows, the marks.
  const CP437 = [
    0x0020, 0x263A, 0x263B, 0x2665, 0x2666, 0x2663, 0x2660, 0x2022, 0x25D8, 0x25CB, 0x25D9, 0x2642, 0x2640, 0x266A, 0x266B, 0x263C,
    0x25BA, 0x25C4, 0x2195, 0x203C, 0x00B6, 0x00A7, 0x25AC, 0x21A8, 0x2191, 0x2193, 0x2192, 0x2190, 0x221F, 0x2194, 0x25B2, 0x25BC,
  ];
  for (let i = 32; i < 127; i++) CP437.push(i);
  CP437.push(0x2302);
  CP437.push(...[
    0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF, 0x00EE, 0x00EC, 0x00C4, 0x00C5,
    0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9, 0x00FF, 0x00D6, 0x00DC, 0x00A2, 0x00A3, 0x00A5, 0x20A7, 0x0192,
    0x00E1, 0x00ED, 0x00F3, 0x00FA, 0x00F1, 0x00D1, 0x00AA, 0x00BA, 0x00BF, 0x2310, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB,
    0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x2561, 0x2562, 0x2556, 0x2555, 0x2563, 0x2551, 0x2557, 0x255D, 0x255C, 0x255B, 0x2510,
    0x2514, 0x2534, 0x252C, 0x251C, 0x2500, 0x253C, 0x255E, 0x255F, 0x255A, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256C, 0x2567,
    0x2568, 0x2564, 0x2565, 0x2559, 0x2558, 0x2552, 0x2553, 0x256B, 0x256A, 0x2518, 0x250C, 0x2588, 0x2584, 0x258C, 0x2590, 0x2580,
    0x03B1, 0x00DF, 0x0393, 0x03C0, 0x03A3, 0x03C3, 0x00B5, 0x03C4, 0x03A6, 0x0398, 0x03A9, 0x03B4, 0x221E, 0x03C6, 0x03B5, 0x2229,
    0x2261, 0x00B1, 0x2265, 0x2264, 0x2320, 0x2321, 0x00F7, 0x2248, 0x00B0, 0x2219, 0x00B7, 0x221A, 0x207F, 0x00B2, 0x25A0, 0x00A0,
  ]);

  // The box-drawing characters as arms - left, right, up, down; 0 none,
  // 1 single, 2 double - drawn as rectangles rather than taken from the
  // font, whose lines do not reach the cell's edges and so do not meet.
  const ARMS = {
    0x2500: [1,1,0,0], 0x2502: [0,0,1,1], 0x250C: [0,1,0,1], 0x2510: [1,0,0,1], 0x2514: [0,1,1,0], 0x2518: [1,0,1,0],
    0x251C: [0,1,1,1], 0x2524: [1,0,1,1], 0x252C: [1,1,0,1], 0x2534: [1,1,1,0], 0x253C: [1,1,1,1],
    0x2550: [2,2,0,0], 0x2551: [0,0,2,2], 0x2552: [0,2,0,1], 0x2553: [0,1,0,2], 0x2554: [0,2,0,2],
    0x2555: [2,0,0,1], 0x2556: [1,0,0,2], 0x2557: [2,0,0,2], 0x2558: [0,2,1,0], 0x2559: [0,1,2,0], 0x255A: [0,2,2,0],
    0x255B: [2,0,1,0], 0x255C: [1,0,2,0], 0x255D: [2,0,2,0], 0x255E: [0,2,1,1], 0x255F: [0,1,2,2], 0x2560: [0,2,2,2],
    0x2561: [2,0,1,1], 0x2562: [1,0,2,2], 0x2563: [2,0,2,2], 0x2564: [2,2,0,1], 0x2565: [1,1,0,2], 0x2566: [2,2,0,2],
    0x2567: [2,2,1,0], 0x2568: [1,1,2,0], 0x2569: [2,2,2,0], 0x256A: [2,2,1,1], 0x256B: [1,1,2,2], 0x256C: [2,2,2,2],
  };

  // A box character into its cell: every arm a bar from the edge towards
  // the centre, and how far past the centre it reaches makes the joins.
  // A double arm is two bars, d apart. Each looks at the arm across it on
  // its own side: a double there and the bar stops d short of the centre
  // (at that arm's inner line); a single, at the centre; none, and it
  // runs d past the centre to meet its opposite or close a corner with a
  // double on the far side. A single arm runs to the centre, or d past
  // it when a double crosses it, so the double's two lines are bridged.
  function drawBox(ctx, x0, y0, w, h, a) {
    const t = Math.max(1, Math.floor(h / 14));
    const d = Math.max(2, Math.floor(h / 8));
    const cx = x0 + Math.floor(w / 2), cy = y0 + Math.floor(h / 2);
    const [l, r, u, dn] = a;
    const bar = (xa, ya, xb, yb) => ctx.fillRect(Math.min(xa, xb), Math.min(ya, yb), Math.abs(xb - xa), Math.abs(yb - ya));
    const reach = (same, other) => same === 2 ? -d : same === 1 ? 0 : (other === 1 ? 0 : d);
    const singleReach = (p, q) => (p === 2 || q === 2) ? d : 0;
    for (const [arm, dir] of [[l, -1], [r, 1]]) {
      if (!arm) continue;
      const edge = dir < 0 ? x0 : x0 + w;
      if (arm === 1) {
        const end = cx - dir * singleReach(u, dn);
        bar(edge, cy, end + (dir < 0 ? t : 0), cy + t);
      } else {
        for (const s of [-1, 1]) {
          const y = cy + s * d;
          const [same, other] = s < 0 ? [u, dn] : [dn, u];
          const end = cx - dir * reach(same, other);
          bar(edge, y, end + (dir < 0 ? t : 0), y + t);
        }
      }
    }
    for (const [arm, dir] of [[u, -1], [dn, 1]]) {
      if (!arm) continue;
      const edge = dir < 0 ? y0 : y0 + h;
      if (arm === 1) {
        const end = cy - dir * singleReach(l, r);
        bar(cx, edge, cx + t, end + (dir < 0 ? t : 0));
      } else {
        for (const s of [-1, 1]) {
          const x = cx + s * d;
          const [same, other] = s < 0 ? [l, r] : [r, l];
          const end = cy - dir * reach(same, other);
          bar(x, edge, x + t, end + (dir < 0 ? t : 0));
        }
      }
    }
  }

  // The three shades as dot patterns, one canvas pattern per colour pair,
  // made when first needed: the dots a DOS card drew.
  const SHADE_ROWS = [[0x22, 0x88], [0xAA, 0x55], [0xDD, 0x77]];
  const patterns = new Map();
  function shadePattern(ctx, which, fg, bg) {
    const key = which + fg + bg;
    let p = patterns.get(key);
    if (p) return p;
    const c = document.createElement('canvas');
    c.width = 8; c.height = 8;
    const g = c.getContext('2d');
    g.fillStyle = bg; g.fillRect(0, 0, 8, 8);
    g.fillStyle = fg;
    for (let y = 0; y < 8; y++) {
      const row = SHADE_ROWS[which][y & 1];
      for (let x = 0; x < 8; x++) if (row & (0x80 >> x)) g.fillRect(x, y, 1, 1);
    }
    p = ctx.createPattern(c, 'repeat');
    patterns.set(key, p);
    return p;
  }

  // The named keys, by the numbers the module (and the wire) uses.
  const NAMED = {
    Enter: 0, Escape: 1, Tab: 2, Backspace: 4, Delete: 5, Insert: 6, Home: 7, End: 8,
    PageUp: 9, PageDown: 10, ArrowUp: 11, ArrowDown: 12, ArrowLeft: 13, ArrowRight: 14,
  };

  async function start(canvas, url) {
    const { instance } = await WebAssembly.instantiateStreaming(fetch(url), {});
    const m = instance.exports;
    const ctx = canvas.getContext('2d', { alpha: false });

    // The cell: a monospace font, measured once. 16 by 9 pixels of a
    // console at 100% is too small on a modern screen; this is what
    // reads well and keeps the 80x25 shape.
    const FONT = '18px "Cascadia Mono", "Consolas", "DejaVu Sans Mono", "Courier New", monospace';
    ctx.font = FONT;
    const cellW = Math.ceil(ctx.measureText('M').width);
    const cellH = 22;
    const dpr = window.devicePixelRatio || 1;

    let cols = 80, rows = 25;
    function fit() {
      cols = Math.max(20, Math.floor(window.innerWidth / cellW));
      rows = Math.max(8, Math.floor(window.innerHeight / cellH));
      canvas.width = cols * cellW * dpr;
      canvas.height = rows * cellH * dpr;
      canvas.style.width = cols * cellW + 'px';
      canvas.style.height = rows * cellH + 'px';
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.font = FONT;
      ctx.textBaseline = 'middle';
    }
    fit();
    m.owl_init(cols, rows);

    let blink = true;
    setInterval(() => { blink = !blink; draw(); }, 500);

    // Draw the frame the module holds now. Every cell every time: a
    // frame is a few thousand fills and it is not worth being clever.
    function draw() {
      if (!m.owl_running()) {
        ctx.fillStyle = '#000';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        document.getElementById('note').textContent = 'The program has ended. Reload the page to start again.';
        return;
      }
      const ptr = m.owl_frame();
      const w = m.owl_width(), h = m.owl_height();
      const cells = new Uint8Array(m.memory.buffer, ptr, w * h * 3);
      let i = 0;
      for (let y = 0; y < h; y++) {
        for (let x = 0; x < w; x++, i += 3) {
          const glyph = cells[i] | (cells[i + 1] << 8);
          const attr = cells[i + 2];
          ctx.fillStyle = PALETTE[attr >> 4];
          ctx.fillRect(x * cellW, y * cellH, cellW, cellH);
          if (glyph === 0 || glyph === 32) continue;
          const code = glyph < 256 ? CP437[glyph] : glyph;
          ctx.fillStyle = PALETTE[attr & 15];
          const ch = String.fromCodePoint(code);
          // Block glyphs are drawn as blocks, not as font characters: a
          // font's ▀ and ▄ leave gaps the DOS card never had.
          if (code === 0x2588) ctx.fillRect(x * cellW, y * cellH, cellW, cellH);
          else if (code === 0x2580) ctx.fillRect(x * cellW, y * cellH, cellW, cellH / 2);
          else if (code === 0x2584) ctx.fillRect(x * cellW, y * cellH + cellH / 2, cellW, cellH / 2);
          else if (code === 0x258C) ctx.fillRect(x * cellW, y * cellH, cellW / 2, cellH);
          else if (code === 0x2590) ctx.fillRect(x * cellW + cellW / 2, y * cellH, cellW / 2, cellH);
          else if (code === 0x2591 || code === 0x2592 || code === 0x2593) {
            ctx.fillStyle = shadePattern(ctx, code - 0x2591, PALETTE[attr & 15], PALETTE[attr >> 4]);
            ctx.fillRect(x * cellW, y * cellH, cellW, cellH);
          } else if (ARMS[code]) {
            drawBox(ctx, x * cellW, y * cellH, cellW, cellH, ARMS[code]);
          } else {
            ctx.fillText(ch, x * cellW, y * cellH + cellH / 2);
          }
        }
      }
      // The caret: a bar at the bottom of its cell, blinking, as the
      // hardware's did.
      const cx = m.owl_cursor_x(), cy = m.owl_cursor_y();
      if (cx >= 0 && cy >= 0 && blink) {
        const at = (cy * w + cx) * 3;
        ctx.fillStyle = PALETTE[cells[at + 2] & 15];
        ctx.fillRect(cx * cellW, (cy + 1) * cellH - 3, cellW, 2);
      }
      // Something is shown chosen: hold this frame a moment, then deliver.
      if (m.owl_hold()) setTimeout(() => { m.owl_tick(); draw(); }, 90);
    }

    // ---- input ----
    window.addEventListener('resize', () => { fit(); m.owl_resize(cols, rows); draw(); });

    document.addEventListener('keydown', e => {
      Owlosui.keys = (Owlosui.keys || 0) + 1;
      let kind, value;
      const mods = (e.shiftKey ? 1 : 0) | (e.ctrlKey ? 2 : 0) | (e.altKey ? 4 : 0);
      if (e.key === 'Tab') { kind = 2; value = e.shiftKey ? 3 : 2; }
      else if (e.key in NAMED) { kind = 2; value = NAMED[e.key]; }
      else if (/^F\d{1,2}$/.test(e.key)) { kind = 1; value = parseInt(e.key.slice(1), 10); }
      else if (e.key.length === 1) {
        kind = 0;
        // With Ctrl or Alt the letter itself is what the core wants: Alt+X
        // is Char('x') with alt, as it is on the wire.
        value = (mods & 6) ? e.key.toLowerCase().codePointAt(0) : e.key.codePointAt(0);
      } else return;
      e.preventDefault();
      m.owl_key(kind, value, mods);
      draw();
    });

    let buttons = 0;
    let lastDown = 0, lastAt = '';
    function cell(e) {
      const r = canvas.getBoundingClientRect();
      return [Math.floor((e.clientX - r.left) / cellW), Math.floor((e.clientY - r.top) / cellH)];
    }
    canvas.addEventListener('contextmenu', e => e.preventDefault());
    canvas.addEventListener('mousedown', e => {
      const [x, y] = cell(e);
      const b = e.button === 2 ? 1 : e.button === 1 ? 2 : 0;
      buttons |= 1 << b;
      // A second press on the same cell within half a second is a double.
      const now = performance.now();
      const at = x + ',' + y + ',' + b;
      const twice = now - lastDown < 500 && at === lastAt;
      lastDown = twice ? 0 : now;
      lastAt = at;
      m.owl_mouse(twice ? 6 : 0, b, x, y);
      draw();
      e.preventDefault();
    });
    canvas.addEventListener('mouseup', e => {
      const [x, y] = cell(e);
      const b = e.button === 2 ? 1 : e.button === 1 ? 2 : 0;
      buttons &= ~(1 << b);
      m.owl_mouse(1, b, x, y);
      draw();
    });
    canvas.addEventListener('mousemove', e => {
      if (!buttons) return;
      const [x, y] = cell(e);
      m.owl_mouse(2, 0, x, y);
      draw();
    });
    canvas.addEventListener('wheel', e => {
      const [x, y] = cell(e);
      m.owl_mouse(e.deltaY < 0 ? 4 : 5, 0, x, y);
      draw();
      e.preventDefault();
    }, { passive: false });

    draw();
    // For a page's own scripts and for tests: the module, and a count of
    // what arrived.
    Owlosui.module = m;
    return m;
  }

  return { start, CP437, PALETTE, module: null, keys: 0 };
})();
