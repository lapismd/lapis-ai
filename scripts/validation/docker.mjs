import { execFileSync, spawn } from "node:child_process";
import {
  closeSync,
  createReadStream,
  createWriteStream,
  openSync,
} from "node:fs";
import { pipeline } from "node:stream/promises";
import { join } from "node:path";

export function dockerCommand(config, args) {
  const context = config.context ? ["--context", config.context] : [];
  if (config.transport !== "ssh-cli") return ["docker", [...context, ...args]];
  const endpoint = new URL(
    execFileSync(
      "docker",
      [
        ...context,
        "context",
        "inspect",
        ...(config.context ? [config.context] : []),
        "--format",
        "{{.Endpoints.docker.Host}}",
      ],
      { encoding: "utf8" },
    ).trim(),
  );
  if (
    endpoint.protocol !== "ssh:" ||
    !endpoint.hostname ||
    !/^[a-zA-Z0-9_.-]+$/.test(decodeURIComponent(endpoint.username)) ||
    endpoint.password
  )
    throw Error("SSH CLI transport needs an SSH Docker context");
  const quote = (value) => "'" + value.replaceAll("'", "'\\''") + "'";
  return [
    "ssh",
    [
      "-T",
      "-o",
      "BatchMode=yes",
      "-o",
      "ConnectTimeout=10",
      "-o",
      "ControlMaster=auto",
      "-o",
      "ControlPersist=60",
      "-o",
      "ControlPath=/tmp/lapis-ai-validation-%C",
      ...(endpoint.port ? ["-p", endpoint.port] : []),
      `${decodeURIComponent(endpoint.username)}@${endpoint.hostname}`,
      ["docker", ...args].map(quote).join(" "),
    ],
  ];
}
export function output(config, args) {
  const [command, argv] = dockerCommand(config, args);
  return execFileSync(command, argv, {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}
export function execute(command, args, options = {}) {
  return new Promise((accept, reject) => {
    const child = spawn(command, args, { stdio: "inherit", ...options });
    child.once("error", reject);
    child.once("exit", (code, signal) =>
      code === 0
        ? accept()
        : reject(Error(`${command} exited ${code ?? signal}`)),
    );
  });
}
export const docker = (config, args) => execute(...dockerCommand(config, args));
export async function saveLogs(config, run, destination) {
  const fd = openSync(destination, "w");
  try {
    await execute(...dockerCommand(config, ["logs", run]), {
      stdio: ["ignore", fd, fd],
    });
  } finally {
    closeSync(fd);
  }
}
const finished = (child) =>
  new Promise((accept, reject) => {
    child.once("error", reject);
    child.once("exit", (code) =>
      code === 0 ? accept() : reject(Error(`Docker transfer exited ${code}`)),
    );
  });
export async function sendDirectory(config, args, directory) {
  const tar = spawn(
    "tar",
    [
      "--format=ustar",
      "--no-xattrs",
      "--no-acls",
      "-C",
      directory,
      "-cf",
      "-",
      ".",
    ],
    { stdio: ["ignore", "pipe", "inherit"] },
  );
  const child = spawn(...dockerCommand(config, args), {
    stdio: ["pipe", "inherit", "inherit"],
  });
  await Promise.all([
    pipeline(tar.stdout, child.stdin),
    finished(tar),
    finished(child),
  ]);
}
export async function receiveDirectory(config, args, directory) {
  const archive = join(directory, "reports.tar"),
    child = spawn(...dockerCommand(config, args), {
      stdio: ["ignore", "pipe", "inherit"],
    });
  // Drain archive padding before extraction; BSD tar may otherwise close early.
  await Promise.all([
    pipeline(child.stdout, createWriteStream(archive)),
    finished(child),
  ]);
  await execute("tar", ["-C", directory, "-xf", archive]);
}
export async function exportPack(root, gitDirectory, revision, destination) {
  const child = spawn(
    "git",
    ["--git-dir", gitDirectory, "pack-objects", "--revs", "--stdout"],
    { cwd: root, stdio: ["pipe", "pipe", "inherit"] },
  );
  child.stdin.end(revision + "\n");
  await Promise.all([
    pipeline(child.stdout, createWriteStream(destination)),
    finished(child),
  ]);
}
export async function importPack(root, source) {
  const child = spawn("git", ["index-pack", "--stdin"], {
    cwd: root,
    stdio: ["pipe", "inherit", "inherit"],
  });
  await Promise.all([
    pipeline(createReadStream(source), child.stdin),
    finished(child),
  ]);
}
