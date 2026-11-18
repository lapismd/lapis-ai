import { lstatSync, readFileSync, realpathSync } from "node:fs";
import { resolve } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { parse } from "yaml";
export const managedMaintenanceNote = (path) =>
  /^\.changeset\/lapis-dependencies-[a-f0-9]{12}\.md$/.test(path);
export function maintenanceNotePaths(changedFiles) {
  const notes = changedFiles.filter((path) =>
    path.startsWith(".changeset/lapis-dependencies-"),
  );
  if (notes.length > 1 || notes.some((path) => !managedMaintenanceNote(path)))
    throw Error("Unexpected managed maintenance path or count");
  return notes;
}
export function readMaintenanceNote(path) {
  const target = resolve(path),
    info = lstatSync(target);
  if (realpathSync(target) !== target || !info.isFile() || info.size > 8192)
    throw Error("Unsafe or oversized managed maintenance file");
  return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(
    readFileSync(target),
  );
}
export function readCurrentMaintenanceNote(path) {
  return lstatSync(resolve(path), { throwIfNoEntry: false }) === undefined
    ? undefined
    : readMaintenanceNote(path);
}
export function loadMaintenanceNotes(root, changedFiles, files) {
  const consumed = [];
  for (const path of maintenanceNotePaths(changedFiles)) {
    const bytes = readCurrentMaintenanceNote(resolve(root, path));
    if (bytes === undefined) consumed.push(path);
    else files[path] = bytes;
  }
  return consumed;
}
export function verifyMaintenanceLifecycle(
  changedFiles,
  files,
  consumedNotes,
  unchangedFacts,
) {
  const notes = maintenanceNotePaths(changedFiles);
  if (
    !Array.isArray(consumedNotes) ||
    new Set(consumedNotes).size !== consumedNotes.length ||
    consumedNotes.some(
      (path) => !notes.includes(path) || Object.hasOwn(files, path),
    )
  )
    throw Error("Unexpected consumed maintenance inventory");
  verifyMaintenanceNotes(
    unchangedFacts
      ? changedFiles.filter((path) => !consumedNotes.includes(path))
      : changedFiles,
    files,
  );
}
export function verifyMaintenanceNotes(changedFiles, files, release = {}) {
  const notes = maintenanceNotePaths(changedFiles);
  for (const path of notes) {
    const bytes = files[path];
    const front =
      typeof bytes === "string" &&
      bytes.isWellFormed() &&
      Buffer.byteLength(bytes) <= 8192 &&
      /^---\r?\n([\s\S]*?)^---[ \t]*(?:\r?\n|$)/m.exec(bytes);
    if (
      !front ||
      front.index !== 0 ||
      !isDeepStrictEqual(front[1].trim() ? parse(front[1]) : {}, release)
    )
      throw Error("Unexpected managed maintenance release metadata");
  }
  return notes;
}
export function verifyMaintenanceScope(changedFiles, files, allowedPaths) {
  const notes = verifyMaintenanceNotes(changedFiles, files);
  if (
    changedFiles.some(
      (path) => !allowedPaths.includes(path) && !notes.includes(path),
    )
  )
    throw Error("Unexpected development intake source scope");
}
