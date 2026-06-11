export type BoundedLruSet = {
  has: (key: string) => boolean;
  add: (key: string) => void;
  clear: () => void;
  readonly size: number;
};

export type BoundedLruMap<K, V> = {
  get: (key: K) => V | undefined;
  set: (key: K, value: V) => void;
  clear: () => void;
  readonly size: number;
};

export const createBoundedLruSet = (maxSize: number): BoundedLruSet => {
  if (maxSize < 1) {
    throw new RangeError('maxSize must be at least 1');
  }

  const entries = new Map<string, true>();

  const touch = (key: string): void => {
    entries.delete(key);
    entries.set(key, true);
  };

  return {
    has(key: string): boolean {
      return entries.has(key);
    },
    add(key: string): void {
      if (entries.has(key)) {
        touch(key);
        return;
      }

      entries.set(key, true);
      if (entries.size > maxSize) {
        const oldestKey = entries.keys().next().value;
        if (oldestKey !== undefined) {
          entries.delete(oldestKey);
        }
      }
    },
    clear(): void {
      entries.clear();
    },
    get size(): number {
      return entries.size;
    },
  };
};

export const createBoundedLruMap = <K, V>(maxSize: number): BoundedLruMap<K, V> => {
  if (maxSize < 1) {
    throw new RangeError('maxSize must be at least 1');
  }

  const entries = new Map<K, V>();

  const touch = (key: K, value: V): void => {
    entries.delete(key);
    entries.set(key, value);
  };

  return {
    get(key: K): V | undefined {
      const value = entries.get(key);
      if (value === undefined) {
        return undefined;
      }

      touch(key, value);
      return value;
    },
    set(key: K, value: V): void {
      if (entries.has(key)) {
        touch(key, value);
        return;
      }

      entries.set(key, value);
      if (entries.size > maxSize) {
        const oldestKey = entries.keys().next().value;
        if (oldestKey !== undefined) {
          entries.delete(oldestKey);
        }
      }
    },
    clear(): void {
      entries.clear();
    },
    get size(): number {
      return entries.size;
    },
  };
};