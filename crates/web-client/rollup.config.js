import rust from "@wasm-tool/rollup-plugin-rust";
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";

// Flag that indicates if the build is meant for development purposes.
// If true, wasm-opt is not applied.
const devMode = process.env.MIDEN_WEB_DEV === "true";
const wasmOptArgs = [
  "-O0", // Always use -O0 to avoid hanging
  "--enable-bulk-memory",
  "--enable-nontrapping-float-to-int",
];

// Base cargo arguments
const baseCargoArgs = [
  "--config",
  `build.rustflags=["-C", "target-feature=+atomics,+bulk-memory,+mutable-globals", "-C", "link-arg=--max-memory=4294967296"]`,
  "--no-default-features",
];

/**
 * Rollup configuration file for building a Cargo project and creating a WebAssembly (WASM) module.
 *
 * The configuration sets up a build process that:
 *
 * 1. **WASM Module Build:**
 *    Compiles Rust code into WASM using the @wasm-tool/rollup-plugin-rust plugin. This process
 *    applies specific cargo arguments to enable necessary WebAssembly features (such as atomics,
 *    bulk memory operations, and mutable globals) and to set maximum memory limits. For testing builds,
 *    the WASM optimization level is set to 0 to improve build times, reducing the feedback loop during development.
 *
 * 2. **Main Entry Point Build:**
 *    Resolves and bundles the main JavaScript file (`index.js`) for the primary entry point of the application
 *    into the `dist` directory.
 *
 * Each build configuration outputs ES module format files with source maps to facilitate easier debugging.
 */
export default [
  // Build the WASM module first
  {
    input: "./js/wasm.js",
    output: {
      dir: `dist`,
      format: "es",
      sourcemap: true,
      assetFileNames: "assets/[name][extname]",
    },
    plugins: [
      rust({
        verbose: true,
        extraArgs: {
          cargo: [...baseCargoArgs],
          wasmOpt: wasmOptArgs,
        },
        experimental: {
          typescriptDeclarationDir: "dist",
        },
        optimize: { release: true, rustc: true },
      }),
      resolve(),
      commonjs(),
    ],
  },
  // Build the main entry point
  {
    input: "./js/index.js",
    output: {
      dir: `dist`,
      format: "es",
      sourcemap: true,
    },
    plugins: [resolve(), commonjs()],
  },
];
