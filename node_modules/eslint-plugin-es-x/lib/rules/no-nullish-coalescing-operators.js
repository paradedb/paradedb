/**
 * @author Yosuke Ota <https://github.com/ota-meshi>
 * See LICENSE file in root directory for full license.
 */
"use strict"

/**
 * Checks if the given token is a nullish coalescing operator or not.
 * @param {Token} token - The token to check.
 * @returns {boolean} `true` if the token is a nullish coalescing operator.
 */
function isNullishCoalescingOperator(token) {
    return token.value === "??" && token.type === "Punctuator"
}

module.exports = {
    meta: {
        docs: {
            description: "disallow nullish coalescing operators.",
            category: "ES2020",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-nullish-coalescing-operators.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2020 nullish coalescing operators are forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "LogicalExpression[operator='??']"(node) {
                context.report({
                    node: context
                        .getSourceCode()
                        .getTokenAfter(node.left, isNullishCoalescingOperator),
                    messageId: "forbidden",
                })
            },
        }
    },
}
