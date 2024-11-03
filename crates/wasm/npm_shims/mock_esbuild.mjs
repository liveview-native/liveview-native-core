import * as esbuild from "esbuild";
import { execSync } from "child_process";

/// Runs a build identical to the one specified in the liveview config.exs
/// except substitutes all out modules
let envPlugin = {
  name: "env",
  setup(build) {
    build.onResolve({ filter: /^env$/ }, (args) => ({
      path: args.path,
      namespace: "env-ns",
    }));

    build.onLoad({ filter: /.*/, namespace: "env-ns" }, () => ({
      contents: JSON.stringify(process.env),
      loader: "json",
    }));
  },
};

// get current tag, remove the leading `v`
const LV_VSN = execSync("git describe --tags --abbrev=0")
  .toString()
  .trim()
  .substring(1);

await esbuild.build({
  entryPoints: ["./assets/js/phoenix_live_view"],
  plugins: [envPlugin],
  bundle: true,
  format: "esm",
  sourcemap: true,
  define: {
    LV_VSN: JSON.stringify(LV_VSN),
  },
  outfile: "./priv/static/phoenix_live_view.esm.js",
});
