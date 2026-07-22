import { access, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packagePath = path.join(repositoryRoot, "package.json");
const helperPath = path.join(repositoryRoot, "scripts", "build-unsigned-installer.ps1");
const packageJson = JSON.parse(await readFile(packagePath, "utf8"));
const scripts = packageJson.scripts ?? {};

const expectedScripts = {
  tauri: "tauri",
  desktop: "bun run tauri dev --features desktop --config backend/tauri.conf.json",
  "dev:desktop": "bun run tauri dev --features desktop --config backend/tauri.conf.json",
  "build:desktop": "bun run tauri build --features desktop --config backend/tauri.conf.json --bundles nsis",
};

for (const [name, expected] of Object.entries(expectedScripts)) {
  if (scripts[name] !== expected) {
    throw new Error(`package.json script '${name}' must be '${expected}'`);
  }
}

const serializedScripts = JSON.stringify(scripts);
if (/cargo\s+tauri/i.test(serializedScripts)) {
  throw new Error("package.json must not invoke host-global cargo-tauri");
}

if (!packageJson.devDependencies?.["@tauri-apps/cli"]) {
  throw new Error("@tauri-apps/cli must remain a project devDependency");
}

const helper = await readFile(helperPath, "utf8");
if (/cargo\s+tauri/i.test(helper)) {
  throw new Error("Unsigned installer helper must not invoke host-global cargo-tauri");
}
for (const invocation of ["bun run tauri build", "bun run tauri bundle"]) {
  if (!helper.includes(invocation)) {
    throw new Error(`Unsigned installer helper is missing '${invocation}'`);
  }
}

const localBinaryNames = process.platform === "win32" ? ["tauri.exe", "tauri.cmd"] : ["tauri"];
let localBinary;
for (const name of localBinaryNames) {
  const candidate = path.join(repositoryRoot, "node_modules", ".bin", name);
  try {
    await access(candidate);
    localBinary = candidate;
    break;
  } catch {
  }
}
if (!localBinary) {
  throw new Error("Repository-local Tauri CLI is missing; run 'bun install' first");
}

console.log(`Local Tauri CLI wiring OK: ${path.relative(repositoryRoot, localBinary)}`);
