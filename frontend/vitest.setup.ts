// Node ≥ 25 ships an experimental global `localStorage` that is `undefined`
// unless the process runs with --localstorage-file; it shadows jsdom's
// implementation in vitest. Install an in-memory Storage when the global is
// inert so browser-targeted code under test behaves like in a browser.
class MemoryStorage implements Storage {
  private data = new Map<string, string>();
  get length() {
    return this.data.size;
  }
  clear() {
    this.data.clear();
  }
  getItem(key: string) {
    return this.data.has(key) ? this.data.get(key)! : null;
  }
  key(index: number) {
    return [...this.data.keys()][index] ?? null;
  }
  removeItem(key: string) {
    this.data.delete(key);
  }
  setItem(key: string, value: string) {
    this.data.set(key, String(value));
  }
}

if (globalThis.localStorage == null) {
  Object.defineProperty(globalThis, "localStorage", {
    value: new MemoryStorage(),
    configurable: true,
  });
}
