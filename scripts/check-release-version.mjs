import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

function fail(message) {
  console.error(`Release validation failed: ${message}`);
  process.exit(1);
}

const tag = process.argv.slice(2).find((argument) => argument !== "--") ?? process.env.GITHUB_REF_NAME;

if (!tag) {
  fail("provide a tag such as v0.1.0");
}

const packageManifest = JSON.parse(readFileSync("package.json", "utf8"));
const tauriConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const cargoMetadata = JSON.parse(
  execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
    encoding: "utf8",
  }),
);
const tauriPackage = cargoMetadata.packages.find(({ name }) => name === "danxi-next");

if (!tauriPackage) {
  fail("cannot find the danxi-next Rust package");
}

const versions = {
  packageJson: packageManifest.version,
  tauriConfig: tauriConfig.version,
  cargoPackage: tauriPackage.version,
};
const uniqueVersions = new Set(Object.values(versions));

if (uniqueVersions.size !== 1) {
  fail(`version mismatch: ${JSON.stringify(versions)}`);
}

const version = packageManifest.version;
const expectedTag = `v${version}`;

if (tag !== expectedTag) {
  fail(`tag ${tag} does not match application version ${expectedTag}`);
}

console.log(`Release tag ${tag} matches DanXi Next ${version}.`);
