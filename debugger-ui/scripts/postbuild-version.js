import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const PACKAGE_JSON = path.resolve("package.json");
const pkgRaw = readFileSync(PACKAGE_JSON, "utf8");
const pkg = JSON.parse(pkgRaw);
pkg.version = "__APP_VERSION__";
writeFileSync(PACKAGE_JSON, `${JSON.stringify(pkg, null, 2)}\n`);
