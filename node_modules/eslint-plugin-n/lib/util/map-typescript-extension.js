"use strict"

const path = require("path")
const isTypescript = require("../util/is-typescript")

const mapping = {
    "": ".js", // default empty extension will map to js
    ".ts": ".js",
    ".cts": ".cjs",
    ".mts": ".mjs",
    ".tsx": ".jsx",
}

const reverseMapping = {
    ".js": ".ts",
    ".cjs": ".cts",
    ".mjs": ".mts",
    ".jsx": ".tsx",
}

/**
 * Maps the typescript file extension that should be added in an import statement,
 * based on the given file extension of the referenced file OR fallsback to the original given extension.
 *
 * For example, in typescript, when referencing another typescript from a typescript file,
 * a .js extension should be used instead of the original .ts extension of the referenced file.
 *
 * @param {RuleContext} context
 * @param {string} filePath The filePath of the import
 * @param {string} fallbackExtension The non-typescript fallback
 * @param {boolean} reverse Execute a reverse path mapping
 * @returns {string} The file extension to append to the import statement.
 */
module.exports = function mapTypescriptExtension(
    context,
    filePath,
    fallbackExtension,
    reverse = false
) {
    const ext = path.extname(filePath)
    if (reverse) {
        if (isTypescript(context) && ext in reverseMapping) {
            return reverseMapping[ext]
        }
    } else {
        if (isTypescript(context) && ext in mapping) {
            return mapping[ext]
        }
    }

    return fallbackExtension
}
