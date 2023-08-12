/**
 * @author Yosuke Ota <https://github.com/ota-meshi>
 * See LICENSE file in root directory for full license.
 */
"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow optional chaining.",
            category: "ES2020",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-optional-chaining.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2020 optional chaining is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        const sourceCode = context.getSourceCode()

        /**
         * Checks if the given token is a `?.` token or not.
         * @param {Token} token The token to check.
         * @returns {boolean} `true` if the token is a `?.` token.
         */
        function isQuestionDotToken(token) {
            return (
                token.value === "?." &&
                (token.type === "Punctuator" || // espree has been parsed well.
                    // espree@7.1.0 doesn't parse "?." tokens well. Therefore, get the string from the source code and check it.
                    sourceCode.getText(token) === "?.")
            )
        }

        return {
            "CallExpression[optional=true]"(node) {
                context.report({
                    node: sourceCode.getTokenAfter(
                        node.callee,
                        isQuestionDotToken,
                    ),
                    messageId: "forbidden",
                })
            },
            "MemberExpression[optional=true]"(node) {
                context.report({
                    node: sourceCode.getTokenAfter(
                        node.object,
                        isQuestionDotToken,
                    ),
                    messageId: "forbidden",
                })
            },
        }
    },
}
