/**
 * @author Yosuke Ota
 * See LICENSE file in root directory for full license.
 */
"use strict"

const utils = require("@eslint-community/eslint-utils")

module.exports = {
    meta: {
        docs: {
            description: "disallow logical assignment operators.",
            category: "ES2021",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-logical-assignment-operators.html",
        },
        fixable: "code",
        messages: {
            forbidden: "ES2021 logical assignment operators are forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        const sourceCode = context.getSourceCode()
        return {
            "AssignmentExpression[operator=/(?:\\|\\||&&|\\?\\?)=/]"(node) {
                const operatorToken = sourceCode.getTokenAfter(node.left)
                context.report({
                    node: operatorToken,
                    messageId: "forbidden",
                    fix(fixer) {
                        if (node.left.type !== "Identifier") {
                            return null
                        }
                        const newOperator = node.operator.slice(-1)
                        const biOperator = node.operator.slice(0, -1)
                        const varText = sourceCode.getText(node.left)

                        const results = [
                            fixer.replaceText(operatorToken, newOperator),
                            fixer.insertTextAfter(
                                operatorToken,
                                ` ${varText} ${biOperator}`,
                            ),
                        ]
                        if (!utils.isParenthesized(node.right, sourceCode)) {
                            results.push(
                                fixer.insertTextBefore(node.right, "("),
                                fixer.insertTextAfter(node.right, ")"),
                            )
                        }
                        return results
                    },
                })
            },
        }
    },
}
