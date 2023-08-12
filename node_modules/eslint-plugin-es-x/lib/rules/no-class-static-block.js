/**
 * @author Yosuke Ota <https://github.com/ota-meshi>
 * See LICENSE file in root directory for full license.
 */
"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow class static block.",
            category: "ES2022",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-class-static-block.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2022 class static block is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            StaticBlock(node) {
                context.report({ node, messageId: "forbidden" })
            },
        }
    },
}
