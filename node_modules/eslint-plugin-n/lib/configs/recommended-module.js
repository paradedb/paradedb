"use strict"

const { commonGlobals, commonRules } = require("./_commons")

module.exports = {
    globals: {
        ...commonGlobals,
        __dirname: "off",
        __filename: "off",
        exports: "off",
        module: "off",
        require: "off",
    },
    parserOptions: {
        ecmaFeatures: { globalReturn: false },
        ecmaVersion: 2021,
        sourceType: "module",
    },
    plugins: ["n"],
    rules: {
        ...commonRules,
        "n/no-unsupported-features/es-syntax": [
            "error",
            { ignores: ["modules"] },
        ],
    },
}
