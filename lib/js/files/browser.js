// This browser's own storage for the page: the Origin Private File System.
//
// Every modern browser keeps a small private disk for each site - Chrome,
// Edge, Firefox and Safari all have it - and a page may make folders and
// files in it without asking anyone. Nothing is uploaded; nothing outside
// this site can see it; clearing the site's data empties it. The name for
// it is OPFS, and a person trying the demo does not need to know that:
// the demo calls it "this browser's storage".
//
// Like every source in this folder: list(dir), read(path), write(path,
// text), with paths as /folder/file.txt and folders ending in /.

const WELCOME = `This file is in this browser's own storage.

Every modern browser keeps a small private disk for each web site. This
page can make files and folders in it; nothing is sent anywhere, and no
other site can see it. Browsers call it OPFS, the Origin Private File
System. Clearing this site's data in the browser's settings empties it.

Try it: change this text, press F2 to save, then open it again from
File > Open from > This browser's storage.
`;

export class BrowserStorage {
  constructor() {
    this.title = "this browser's storage";
    this.about =
      'Files this browser keeps for this page, on this computer. Nothing is uploaded, and it works offline.';
    this.prefix = 'browser:';
  }

  async root() {
    if (!globalThis.navigator?.storage?.getDirectory) {
      throw new Error("This browser has no storage for a page's files. Chrome, Edge, Firefox and Safari have it.");
    }
    return navigator.storage.getDirectory();
  }

  async folder(dir, create = false) {
    let h = await this.root();
    for (const part of dir.split('/').filter(Boolean)) h = await h.getDirectoryHandle(part, { create });
    return h;
  }

  async list(dir) {
    const folder = await this.folder(dir);
    const out = [];
    for await (const [name, h] of folder.entries()) {
      if (h.kind === 'directory') out.push({ name, size: 0, date: new Date(), dir: true });
      else {
        const f = await h.getFile();
        out.push({ name, size: f.size, date: new Date(f.lastModified), dir: false });
      }
    }
    // An empty storage says nothing to someone seeing it for the first
    // time: the first visit finds a file explaining where it is.
    if (dir === '/' && out.length === 0) {
      await this.write('/WELCOME.TXT', WELCOME);
      return this.list(dir);
    }
    return out;
  }

  async read(path) {
    const { dir, name } = split(path);
    const h = await (await this.folder(dir)).getFileHandle(name);
    return (await h.getFile()).text();
  }

  async write(path, text) {
    const { dir, name } = split(path);
    const h = await (await this.folder(dir, true)).getFileHandle(name, { create: true });
    if (!h.createWritable) {
      throw new Error('This browser can read its storage but not save to it from a page yet.');
    }
    const w = await h.createWritable();
    await w.write(text);
    await w.close();
  }
}

/** /a/b/c.txt into its folder, /a/b/, and its name, c.txt. */
export function split(path) {
  const cut = path.lastIndexOf('/');
  return { dir: path.slice(0, cut + 1) || '/', name: path.slice(cut + 1) };
}
