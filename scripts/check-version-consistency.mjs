import { readFileSync } from "node:fs";

const packageJson = JSON.parse(readFileSync("package.json", "utf8"));
const tauriConfig = JSON.parse(readFileSync("backend/tauri.conf.json", "utf8"));
const cargoToml = readFileSync("backend/Cargo.toml", "utf8");
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

const versions = {
  package: packageJson.version,
  cargo: cargoVersion,
  tauri: tauriConfig.version,
};

const uniqueVersions = new Set(Object.values(versions));

if (uniqueVersions.size !== 1 || [...uniqueVersions][0] !== "0.1.0") {
  console.error("Version mismatch:", versions);
  process.exit(1);
}

console.log(`Version consistency OK: ${versions.package}`);
