/**
 * @fileoverview the `recommended-script` config for `eslint.config.js`
 * @author 唯然<weiran.zsd@outlook.com>
 */

"use strict"

const mod = require("../lib/index.js")

module.exports = {
    plugins: { n: mod },
    languageOptions: {
        sourceType: "commonjs",
        globals: mod.configs["recommended-script"].globals,
    },
    rules: mod.configs["recommended-script"].rules,
}
