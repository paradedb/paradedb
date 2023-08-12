/**
 * @author Yosuke Ota
 * See LICENSE file in root directory for full license.
 */
"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow top-level `await`.",
            category: "ES2022",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-top-level-await.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2022 top-level 'await' is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        let functionDepth = 0
        return {
            ":function"() {
                functionDepth++
            },
            ":function:exit"() {
                functionDepth--
            },
            "AwaitExpression, ForOfStatement[await=true]"(node) {
                if (functionDepth > 0) {
                    // not top-level
                    return
                }
                context.report({ node, messageId: "forbidden" })
            },
        }
    },
}
