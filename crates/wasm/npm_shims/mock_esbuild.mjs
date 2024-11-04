import * as esbuild from "esbuild";
import { execSync } from "child_process";

const mockPlugin = {
  name: "mockWasm",
  setup(build) {
    build.onResolve({ filter: /^phoenix_live_view\/rendered$/ }, (args) => {
      return { path: args.path, namespace: "mocked" };
    });

    build.onLoad({ filter: /.*/, namespace: "mocked" }, async () => {
      const wasmProxy = require("liveview_native_core_wasm");
      const renderedExport = wasmProxy.Rendered;

      return {
        contents: `
          const Rendered = ${JSON.stringify(renderedExport)};
          export default Rendered;
        `,
        loader: "js",
      };
    });
  },
};

// get current tag, remove the leading `v`
const LV_VSN = execSync("git describe --tags --abbrev=0")
  .toString()
  .trim()
  .substring(1);

await esbuild.build({
  entryPoints: ["index.js"],
  plugins: [mockPlugin],
  bundle: true,
  format: "esm",
  sourcemap: true,
  define: {
    LV_VSN: JSON.stringify(LV_VSN),
  },
  outfile: "../priv/static/phoenix_live_view.esm.js",
});
