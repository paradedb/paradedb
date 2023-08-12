/**
 * @author Toru Nagashima
 * See LICENSE file in root directory for full license.
 */
"use strict"

const path = require("path")
const isCoreModule = require("is-core-module")
const getResolvePaths = require("./get-resolve-paths")
const getTryExtensions = require("./get-try-extensions")
const ImportTarget = require("./import-target")
const stripImportPathParams = require("./strip-import-path-params")

/**
 * Gets a list of `import`/`export` declaration targets.
 *
 * Core modules of Node.js (e.g. `fs`, `http`) are excluded.
 *
 * @param {RuleContext} context - The rule context.
 * @param {Object} [options] - The flag to include core modules.
 * @param {boolean} [options.includeCore] - The flag to include core modules.
 * @param {number} [options.optionIndex] - The index of rule options.
 * @param {boolean} [options.ignoreTypeImport] - The flag to ignore typescript type imports.
 * @param {function(ImportTarget[]):void} callback The callback function to get result.
 * @returns {ImportTarget[]} A list of found target's information.
 */
module.exports = function visitImport(
    context,
    { includeCore = false, optionIndex = 0, ignoreTypeImport = false } = {},
    callback
) {
    const targets = []
    const basedir = path.dirname(path.resolve(context.getFilename()))
    const paths = getResolvePaths(context, optionIndex)
    const extensions = getTryExtensions(context, optionIndex)
    const options = { basedir, paths, extensions }

    return {
        [[
            "ExportAllDeclaration",
            "ExportNamedDeclaration",
            "ImportDeclaration",
            "ImportExpression",
        ]](node) {
            const sourceNode = node.source

            // skip `import(foo)`
            if (
                node.type === "ImportExpression" &&
                sourceNode &&
                sourceNode.type !== "Literal"
            ) {
                return
            }

            // skip `import type { foo } from 'bar'` (for eslint-typescript)
            if (
                ignoreTypeImport &&
                node.type === "ImportDeclaration" &&
                node.importKind === "type"
            ) {
                return
            }

            const name = sourceNode && stripImportPathParams(sourceNode.value)
            // Note: "999" arbitrary to check current/future Node.js version
            if (name && (includeCore || !isCoreModule(name, "999"))) {
                targets.push(
                    new ImportTarget(sourceNode, name, options, "import")
                )
            }
        },

        "Program:exit"() {
            callback(targets)
        },
    }
}
