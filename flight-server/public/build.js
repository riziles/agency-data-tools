import * as esbuild from "esbuild";

await esbuild.build({
  entryPoints: ["src/app.js"],
  bundle: true,
  outfile: "dist/bundle.js",
  format: "esm",
  platform: "browser",
  target: "es2022",
});
