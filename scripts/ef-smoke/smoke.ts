// Installs nothing itself: run from a folder where `takumi-js` resolves to an espaciofuturo
// release (FORK.md "Release"). Fails unless the fork's native core renders a 3D transform.
import { createRequire } from "node:module";
import { render } from "takumi-js";

const require = createRequire(import.meta.url);
const core = require("@takumi-rs/core/package.json");
const js = require("takumi-js/package.json");
if (!String(core.version).includes("-ef.") || !String(js.version).includes("-ef.")) {
  throw new Error(
    `expected espaciofuturo builds, got takumi-js ${js.version}, @takumi-rs/core ${core.version}`,
  );
}

const card = (transform: string) =>
  `<div style="display:flex;width:400px;height:300px;align-items:center;justify-content:center;background:#f0f0f0;perspective:600px">
    <div style="width:240px;height:160px;background:#0080ff;transform:${transform}"></div>
  </div>`;
const options = { width: 400, height: 300, format: "png" as const };
const flat = await render(card("none"), options);
const tilted = await render(card("perspective(600px) rotateY(35deg)"), options);
const spun = await render(card("rotateY(35deg)"), options);

if (Buffer.compare(Buffer.from(flat), Buffer.from(tilted)) === 0)
  throw new Error("perspective() rendered flat");
if (Buffer.compare(Buffer.from(flat), Buffer.from(spun)) === 0)
  throw new Error("perspective property rendered flat");
console.log(
  `ok: takumi-js ${js.version}, core ${core.version}, ${process.platform}-${process.arch}`,
);
