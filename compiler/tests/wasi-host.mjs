import { WASI } from "node:wasi";
import { readFile } from "node:fs/promises";

const wasi = new WASI({
  version: "preview1",
  args: ["program", ...process.argv.slice(3)],
  env: process.env,
  preopens: { "/": "/" },
});

const bytes = await readFile(process.argv[2]);
const module = await WebAssembly.compile(bytes);
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());

try {
  process.exitCode = wasi.start(instance) ?? 0;
} catch (error) {
  process.stderr.write(`wasi host: ${error.message}\n`);
  process.exitCode = 70;
}
