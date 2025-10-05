import { readFileSync, writeFileSync } from "node:fs";
import { execSync } from "node:child_process";
import path from "node:path";

const PACKAGE_JSON = path.resolve("package.json");

function getGitRevision() {
  try {
    const out = execSync("git rev-parse --short HEAD", { stdio: "pipe" }).toString().trim();
    return out;
  } catch (error) {
    return "unknown";
  }
}

const baseVersion = "2.0.0-pre";
const revision = getGitRevision();
const version = revision === "unknown" ? baseVersion : `${baseVersion}+${revision}`;

const pkgRaw = readFileSync(PACKAGE_JSON, "utf8");
const pkg = JSON.parse(pkgRaw);
pkg.version = version;
writeFileSync(PACKAGE_JSON, `${JSON.stringify(pkg, null, 2)}\n`);
