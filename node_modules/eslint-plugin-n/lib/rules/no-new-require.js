/**
 * @author Wil Moore III
 * See LICENSE file in root directory for full license.
 */
"use strict"

module.exports = {
    meta: {
        type: "suggestion",
        docs: {
            description: "disallow `new` operators with calls to `require`",
            category: "Possible Errors",
            recommended: false,
            url: "https://github.com/weiran-zsd/eslint-plugin-node/blob/HEAD/docs/rules/no-new-require.md",
        },
        fixable: null,
        schema: [],
        messages: {
            noNewRequire: "Unexpected use of new with require.",
        },
    },

    create(context) {
        return {
            NewExpression(node) {
                if (
                    node.callee.type === "Identifier" &&
                    node.callee.name === "require"
                ) {
                    context.report({
                        node,
                        messageId: "noNewRequire",
                    })
                }
            },
        }
    },
}
