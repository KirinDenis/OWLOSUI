// A folder on the web server, read and saved over plain HTTP.
//
// The server here is RUN.CMD's own (Examples/Web/httpd): it shares one
// folder, Examples/Web/files, at /files/. A folder's names come back as
// JSON when it is asked with ?list; a file is a GET; saving is a PUT. Any
// server that answers the same three things will do.
//
// Like every source in this folder: list(dir), read(path), write(path,
// text), with paths as /folder/file.txt and folders ending in /.

export class ServerFolder {
  /** base: where the folder is, e.g. http://localhost:8765/files */
  constructor(base = `${globalThis.location?.origin ?? 'http://localhost:8765'}/files`) {
    this.base = base.replace(/\/+$/, '');
    this.title = "the server's folder";
    this.about = 'Examples/Web/files on the computer running RUN.CMD, read and saved over HTTP by its little server.';
    this.prefix = 'server:';
  }

  url(path) {
    return this.base + path.split('/').map(encodeURIComponent).join('/');
  }

  async fetch(path, init, suffix = '') {
    let r;
    try {
      r = await fetch(this.url(path) + suffix, init);
    } catch {
      throw new Error(`The server did not answer at ${this.base}. Is RUN.CMD still running?`);
    }
    if (!r.ok) throw new Error(`The server said ${r.status} ${r.statusText} for ${path}.`);
    return r;
  }

  async list(dir) {
    const r = await this.fetch(dir, undefined, '?list');
    return (await r.json()).map(e => ({ name: e.name, size: e.size, date: new Date(e.modified * 1000), dir: e.dir }));
  }

  async read(path) {
    return (await this.fetch(path)).text();
  }

  async write(path, text) {
    await this.fetch(path, { method: 'PUT', body: text, headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
  }
}
