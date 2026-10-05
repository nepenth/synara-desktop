#!/usr/bin/env node

// Exit 0 for the same complete pair, 1 for a difference, and 2 for a read
// failure. Ignore timestamps so publication can preserve existing Xcode inputs,
// but compare permissions and link targets without following symbolic links.
import {
  closeSync,
  lstatSync,
  openSync,
  readdirSync,
  readlinkSync,
  readSync,
} from "node:fs";

function metadata(path) {
  try {
    return lstatSync(path);
  } catch (error) {
    if (error.code === "ENOENT" || error.code === "ENOTDIR") return undefined;
    throw error;
  }
}

function sameFile(left, right, size) {
  const leftFile = openSync(left, "r");
  let rightFile;
  try {
    rightFile = openSync(right, "r");
    const leftBytes = Buffer.allocUnsafe(1024 * 1024);
    const rightBytes = Buffer.allocUnsafe(leftBytes.length);
    for (let offset = 0; offset < size;) {
      const length = Math.min(leftBytes.length, size - offset);
      const leftRead = readSync(leftFile, leftBytes, 0, length, offset);
      const rightRead = readSync(rightFile, rightBytes, 0, length, offset);
      if (leftRead !== length || rightRead !== length) {
        throw new Error("generated artifact changed while comparing");
      }
      if (!leftBytes.subarray(0, length).equals(rightBytes.subarray(0, length)))
        return false;
      offset += length;
    }
    return true;
  } finally {
    closeSync(leftFile);
    if (rightFile !== undefined) closeSync(rightFile);
  }
}

function samePath(left, right) {
  const leftInfo = metadata(left);
  const rightInfo = metadata(right);
  if (!leftInfo || !rightInfo || leftInfo.mode !== rightInfo.mode) return false;
  if (leftInfo.isSymbolicLink())
    return readlinkSync(left) === readlinkSync(right);
  if (leftInfo.isFile()) {
    return (
      leftInfo.size === rightInfo.size && sameFile(left, right, leftInfo.size)
    );
  }
  if (leftInfo.isDirectory()) {
    const leftNames = readdirSync(left).sort();
    const rightNames = readdirSync(right).sort();
    return (
      leftNames.length === rightNames.length &&
      leftNames.every(
        (name, index) =>
          name === rightNames[index] &&
          samePath(`${left}/${name}`, `${right}/${name}`),
      )
    );
  }
  throw new Error(`unsupported generated artifact type: ${left}`);
}

try {
  const paths = process.argv.slice(2);
  if (paths.length !== 4)
    throw new Error("expected source/destination Swift and framework paths");
  process.exitCode =
    samePath(paths[0], paths[1]) && samePath(paths[2], paths[3]) ? 0 : 1;
} catch (error) {
  console.error(
    `publish-generated-apple-pair: cannot compare artifacts: ${error.message}`,
  );
  process.exitCode = 2;
}
