/**
 * @author Yosuke Ota <https://github.com/ota-meshi>
 * See LICENSE file in root directory for full license.
 */
"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow `export * as ns`.",
            category: "ES2020",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-export-ns-from.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2020 'export * as ns' are forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "ExportAllDeclaration[exported!=null]"(node) {
                context.report({ node, messageId: "forbidden" })
            },
        }
    },
}
