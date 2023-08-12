"use strict"

module.exports = {
    commonGlobals: {
        // ECMAScript
        ArrayBuffer: "readonly",
        Atomics: "readonly",
        BigInt: "readonly",
        BigInt64Array: "readonly",
        BigUint64Array: "readonly",
        DataView: "readonly",
        Float32Array: "readonly",
        Float64Array: "readonly",
        Int16Array: "readonly",
        Int32Array: "readonly",
        Int8Array: "readonly",
        Map: "readonly",
        Promise: "readonly",
        Proxy: "readonly",
        Reflect: "readonly",
        Set: "readonly",
        SharedArrayBuffer: "readonly",
        Symbol: "readonly",
        Uint16Array: "readonly",
        Uint32Array: "readonly",
        Uint8Array: "readonly",
        Uint8ClampedArray: "readonly",
        WeakMap: "readonly",
        WeakSet: "readonly",

        // ECMAScript (experimental)
        globalThis: "readonly",

        // ECMA-402
        Intl: "readonly",

        // Web Standard
        TextDecoder: "readonly",
        TextEncoder: "readonly",
        URL: "readonly",
        URLSearchParams: "readonly",
        WebAssembly: "readonly",
        clearInterval: "readonly",
        clearTimeout: "readonly",
        console: "readonly",
        queueMicrotask: "readonly",
        setInterval: "readonly",
        setTimeout: "readonly",

        // Node.js
        Buffer: "readonly",
        GLOBAL: "readonly",
        clearImmediate: "readonly",
        global: "readonly",
        process: "readonly",
        root: "readonly",
        setImmediate: "readonly",
    },
    commonRules: {
        "n/no-deprecated-api": "error",
        "n/no-extraneous-import": "error",
        "n/no-extraneous-require": "error",
        "n/no-exports-assign": "error",
        "n/no-missing-import": "error",
        "n/no-missing-require": "error",
        "n/no-process-exit": "error",
        "n/no-unpublished-bin": "error",
        "n/no-unpublished-import": "error",
        "n/no-unpublished-require": "error",
        "n/no-unsupported-features/es-builtins": "error",
        "n/no-unsupported-features/es-syntax": "error",
        "n/no-unsupported-features/node-builtins": "error",
        "n/process-exit-as-throw": "error",
        "n/shebang": "error",
    },
}
