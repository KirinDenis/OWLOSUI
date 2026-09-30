// The calculator's arithmetic, with no screen in it - the same as
// Examples/Desktop/CSharp/05-OwlosDemo/Calculator/CalcEngine.cs, and held to
// the same tests (test.mjs, beside the examples).
//
// A string goes in, a string comes out:
//
//   1 + 2 * 3          precedence: * and / before + and -
//   (1 + 2) * 3        parentheses
//   2 ^ 10             power, right-associative: 2^3^2 is 2^9
//   17 mod 5           remainder, also 17 % 5
//   -3 ^ 2             unary minus binds looser than ^: this is -9
//   sqrt(2)  sin(30)   functions; trig in degrees or radians
//   pi  e  m           constants, and the memory
//   5!                 factorial
//   FF + 1             in hex mode; 1010 in binary; 17 in octal
//
// Numbers are read and written in the current base; outside decimal the
// answer is cut to its integer part. Errors are words: "Divide by zero",
// "Invalid input", "Overflow", "Error".

export const Base = { Dec: 10, Hex: 16, Bin: 2, Oct: 8 };

export class CalcError extends Error {}

const LONG_MAX = 0x7FFFFFFFFFFFFFFFn;

export class CalcEngine {
  constructor() {
    this.base = Base.Dec;
    this.degrees = true;
    this.memory = 0;
  }

  get hasMemory() { return this.memory !== 0; }

  /** The answer as the display should show it, or the error as words. */
  evaluate(expression) {
    try {
      return this.format(this.eval(expression));
    } catch (e) {
      if (e instanceof CalcError) return e.message;
      throw e;
    }
  }

  /** The answer as a number; throws CalcError when there is none. */
  eval(expression) {
    const v = new Parser(expression, this).parseAll();
    if (Number.isNaN(v)) throw new CalcError('Invalid input');
    if (!Number.isFinite(v)) throw new CalcError('Overflow');
    return v;
  }

  /** A number written in the current base. */
  format(v) {
    if (this.base === Base.Dec) {
      // Closer to zero than a double's own noise is zero: sin(180) is
      // 1.2e-16 and nobody wants to read that.
      if (Math.abs(v) < 1e-14) return '0';
      const mag = Math.abs(v);
      if (mag >= 1e15 || mag < 1e-9) {
        // Fifteen significant digits, the trailing zeros dropped, the
        // exponent as C# writes it: 1.18059162071741E+21.
        const [m, e] = v.toPrecision(15).split('e');
        return m.replace(/\.?0+$/, '') + 'E' + (e.startsWith('-') ? e : '+' + e.replace('+', ''));
      }
      const digits = Math.min(15, Math.max(0, 14 - Math.floor(Math.log10(mag))));
      // Rounded half away from zero at that many decimals, then written
      // without trailing zeros.
      const f = 10 ** digits;
      const r = Math.sign(v) * Math.round(Math.abs(v) * f) / f;
      let s = r.toFixed(digits);
      if (s.includes('.')) s = s.replace(/0+$/, '').replace(/\.$/, '');
      return s === '-0' ? '0' : s;
    }
    if (Math.abs(v) >= 9.2e18) throw new CalcError('Overflow');
    const n = BigInt(Math.trunc(v));
    const s = (n < 0n ? -n : n).toString(this.base).toUpperCase();
    return n < 0n ? '-' + s : s;
  }

  /** Whether a character is a digit of the current base. */
  isDigit(c) {
    const d = digit(c);
    return d !== null && d < this.base;
  }
}

function digit(c) {
  if (c >= '0' && c <= '9') return c.charCodeAt(0) - 48;
  const u = c.toUpperCase();
  if (u >= 'A' && u <= 'F' && u.length === 1) return u.charCodeAt(0) - 55;
  return null;
}

const isWordChar = c => /[\p{L}\p{N}]/u.test(c);

/**
 * Recursive descent, one method per level of precedence, loosest first:
 *
 *   sum     := product (('+' | '-') product)*
 *   product := unary (('*' | '/' | 'mod' | '%') unary)*
 *   unary   := ('-' | '+') unary | power
 *   power   := postfix ('^' unary)?
 *   postfix := atom ('!')*
 *   atom    := number | '(' sum ')' | name '(' sum ')' | name
 */
class Parser {
  constructor(text, engine) {
    this.s = text;
    this.e = engine;
    this.i = 0;
  }

  parseAll() {
    this.skip();
    if (this.i >= this.s.length) throw new CalcError('');
    const v = this.sum();
    this.skip();
    if (this.i < this.s.length) throw new CalcError('Error');
    return v;
  }

  skip() { while (this.i < this.s.length && this.s[this.i] === ' ') this.i++; }

  take(c) {
    this.skip();
    if (this.s[this.i] === c) { this.i++; return true; }
    return false;
  }

  takeWord(w) {
    this.skip();
    if (this.s.substr(this.i, w.length).toLowerCase() !== w) return false;
    const end = this.i + w.length;
    if (end < this.s.length && isWordChar(this.s[end])) return false;
    this.i = end;
    return true;
  }

  sum() {
    let v = this.product();
    for (;;) {
      if (this.take('+')) v += this.product();
      else if (this.take('-')) v -= this.product();
      else return v;
    }
  }

  product() {
    let v = this.unary();
    for (;;) {
      if (this.take('*')) v *= this.unary();
      else if (this.take('/')) {
        const d = this.unary();
        if (d === 0) throw new CalcError('Divide by zero');
        v /= d;
      } else if (this.take('%') || this.takeWord('mod')) {
        const d = this.unary();
        if (d === 0) throw new CalcError('Divide by zero');
        v %= d;
      } else return v;
    }
  }

  unary() {
    if (this.take('-')) return -this.unary();
    if (this.take('+')) return this.unary();
    return this.power();
  }

  power() {
    const v = this.postfix();
    // Right-associative, and the exponent may carry its own sign: 2^-1.
    return this.take('^') ? Math.pow(v, this.unary()) : v;
  }

  postfix() {
    let v = this.atom();
    while (this.take('!')) {
      if (v < 0 || v !== Math.floor(v)) throw new CalcError('Invalid input');
      if (v > 170) throw new CalcError('Overflow');
      let f = 1;
      for (let k = 2; k <= v; k++) f *= k;
      v = f;
    }
    return v;
  }

  atom() {
    this.skip();
    if (this.i >= this.s.length) throw new CalcError('Error');
    if (this.take('(')) {
      const v = this.sum();
      if (!this.take(')')) throw new CalcError('Error');
      return v;
    }
    // A run of letters and digits is a number if every character is a
    // digit of the base - in hex, "ace" is a number and "cos" is not.
    const start = this.i;
    while (this.i < this.s.length && (isWordChar(this.s[this.i]) || this.s[this.i] === '.')) this.i++;
    if (this.i === start) throw new CalcError('Error');
    const word = this.s.slice(start, this.i);
    if ([...word].every(c => this.e.isDigit(c) || c === '.')) return this.number(word);
    return this.name(word.toLowerCase());
  }

  number(word) {
    if (this.e.base === Base.Dec) {
      // What a double parses: digits with at most one point.
      if (!/^(\d+\.?\d*|\.\d+)$/.test(word)) throw new CalcError('Error');
      return parseFloat(word);
    }
    if (word.includes('.')) throw new CalcError('Invalid input');
    let n = 0n;
    for (const c of word) {
      n = n * BigInt(this.e.base) + BigInt(digit(c));
      if (n > LONG_MAX) throw new CalcError('Overflow');
    }
    return Number(n);
  }

  name(name) {
    if (name === 'pi') return Math.PI;
    if (name === 'e') return Math.E;
    if (name === 'm') return this.e.memory;
    if (!this.take('(')) throw new CalcError('Error');
    const x = this.sum();
    if (!this.take(')')) throw new CalcError('Error');
    const rad = v => (this.e.degrees ? (v * Math.PI) / 180 : v);
    const deg = v => (this.e.degrees ? (v * 180) / Math.PI : v);
    const positive = (v, f) => { if (v <= 0) throw new CalcError('Invalid input'); return f(v); };
    switch (name) {
      case 'sin': return Math.sin(rad(x));
      case 'cos': return Math.cos(rad(x));
      case 'tan': return Math.tan(rad(x));
      case 'asin': return deg(Math.asin(x));
      case 'acos': return deg(Math.acos(x));
      case 'atan': return deg(Math.atan(x));
      case 'sqrt': if (x < 0) throw new CalcError('Invalid input'); return Math.sqrt(x);
      case 'ln': return positive(x, Math.log);
      case 'log': return positive(x, Math.log10);
      case 'log2': return positive(x, Math.log2);
      case 'exp': return Math.exp(x);
      case 'abs': return Math.abs(x);
      case 'floor': return Math.floor(x);
      case 'ceil': return Math.ceil(x);
      case 'round': return Math.sign(x) * Math.round(Math.abs(x));
      case 'sqr': return x * x;
      default: throw new CalcError('Error');
    }
  }
}
