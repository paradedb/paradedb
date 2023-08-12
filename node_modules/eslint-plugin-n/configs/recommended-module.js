/**
 * @fileoverview the `recommended-module` config for `eslint.config.js`
 * @author 唯然<weiran.zsd@outlook.com>
 */

"use strict"

const mod = require("../lib/index.js")

module.exports = {
    plugins: { n: mod },
    languageOptions: {
        sourceType: "module",
        globals: mod.configs["recommended-module"].globals,
    },
    rules: mod.configs["recommended-module"].rules,
}
