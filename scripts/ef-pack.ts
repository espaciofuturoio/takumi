// Packs the four JS packages of an espaciofuturo release into tarballs that install by URL.
// FORK.md "Release" explains the layout. Run after the helpers, napi (one or more targets),
// wasm and takumi-js builds have filled their package folders:
//   bun scripts/ef-pack.ts --version 2.14.0-ef.1 --out dist-ef [--base-url https://…/download/v2.14.0-ef.1]
import { $ } from "bun";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  options: {
    version: { type: "string" },
    out: { type: "string", default: "dist-ef" },
    "base-url": { type: "string" },
    repo: { type: "string", default: "espaciofuturoio/takumi" },
  },
});

const version = values.version;
if (!version || !/^\d+\.\d+\.\d+-ef\.\d+$/.test(version)) {
  throw new Error(`--version must look like 2.14.0-ef.1, got ${version}`);
}
const baseUrl =
  values["base-url"] ?? `https://github.com/${values.repo}/releases/download/v${version}`;
const root = resolve(import.meta.dir, "..");
const out = resolve(root, values.out!);

const packages = [
  { dir: "takumi-helpers", name: "@takumi-rs/helpers" },
  { dir: "takumi-napi", name: "@takumi-rs/core" },
  { dir: "takumi-wasm", name: "@takumi-rs/wasm" },
  { dir: "takumi-js", name: "takumi-js" },
] as const;

const assetName = (name: string) => `${name.replace("@", "").replace("/", "-")}-${version}.tgz`;
const assetUrl = (name: string) => `${baseUrl}/${assetName(name)}`;
const ours = new Map(packages.map((pkg) => [pkg.name, assetUrl(pkg.name)]));

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });

for (const pkg of packages) {
  const source = join(root, pkg.dir);
  const manifest = JSON.parse(readFileSync(join(source, "package.json"), "utf8"));
  const staging = mkdtempSync(join(tmpdir(), "ef-pack-"));

  manifest.version = version;
  for (const field of ["dependencies", "peerDependencies"] as const) {
    for (const dep of Object.keys(manifest[field] ?? {})) {
      const url = ours.get(dep);
      if (url) manifest[field][dep] = url;
      else if (String(manifest[field][dep]).startsWith("workspace:")) {
        throw new Error(
          `${pkg.name}: ${field}.${dep} is a workspace dependency this script does not pack`,
        );
      }
    }
  }
  delete manifest.devDependencies;
  delete manifest.scripts;
  manifest.repository = { type: "git", url: `git+https://github.com/${values.repo}.git` };

  if (pkg.name === "@takumi-rs/core") {
    // One tarball carries every built binary; the napi loader requires `../core.<target>.node`
    // from dist before it falls back to the per-platform packages, which we do not publish.
    delete manifest.optionalDependencies;
    const binaries = readdirSync(source).filter((file) => /^core\..+\.node$/.test(file));
    if (binaries.length === 0) throw new Error("takumi-napi: no core.*.node binary built");
    manifest.files = [...manifest.files, ...binaries];
    console.log(`@takumi-rs/core binaries: ${binaries.join(", ")}`);
  }

  for (const entry of [...manifest.files, "README.md", "LICENSE-MIT", "LICENSE-APACHE"]) {
    const from = join(source, entry);
    if (existsSync(from)) cpSync(from, join(staging, entry), { recursive: true });
    else if (!["README.md", "LICENSE-MIT", "LICENSE-APACHE"].includes(entry)) {
      throw new Error(`${pkg.name}: ${entry} is in "files" but was not built`);
    }
  }
  for (const licence of ["LICENSE-MIT", "LICENSE-APACHE"]) {
    if (!existsSync(join(staging, licence))) cpSync(join(root, licence), join(staging, licence));
  }
  writeFileSync(join(staging, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);

  await $`bun pm pack --destination ${out} --filename ${assetName(pkg.name)} --ignore-scripts`
    .cwd(staging)
    .quiet();
  rmSync(staging, { recursive: true, force: true });
  console.log(`packed ${assetName(pkg.name)}`);
}
