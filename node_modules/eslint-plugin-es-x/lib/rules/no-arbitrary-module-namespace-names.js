/**
 * @author Yosuke Ota <https://github.com/ota-meshi>
 * See LICENSE file in root directory for full license.
 */
"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow arbitrary module namespace names.",
            category: "ES2022",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-arbitrary-module-namespace-names.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2022 arbitrary module namespace names are forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "ExportAllDeclaration > Literal.exported, ExportSpecifier > Literal.local, ExportSpecifier > Literal.exported, ImportSpecifier > Literal.imported"(
                node,
            ) {
                context.report({
                    node,
                    messageId: "forbidden",
                })
            },
        }
    },
}
