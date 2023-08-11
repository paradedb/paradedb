import resolve from "@rollup/plugin-node-resolve"
import commonjs from "@rollup/plugin-commonjs"
import typescript from "@rollup/plugin-typescript"

export default {
  input: {
    index: "src/index.ts",
    "helpers/index": "src/helpers/index.ts",
    "components/index": "src/components/index.ts",
  },
  output: [
    {
      dir: "dist",
      format: "cjs",
      sourcemap: true,
      entryFileNames: "[name].cjs.js",
    },
    {
      dir: "dist",
      format: "es",
      sourcemap: true,
      entryFileNames: "[name].esm.js",
    },
  ],
  plugins: [typescript(), resolve(), commonjs()],
  external: ["react", "react-dom"],
}
