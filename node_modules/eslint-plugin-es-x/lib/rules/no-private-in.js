/**
 * @author Yosuke Ota <https://github.com/ota-meshi>
 * See LICENSE file in root directory for full license.
 */
"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow `#x in obj`.",
            category: "ES2022",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-private-in.html",
        },
        fixable: null,
        messages: {
            forbidden:
                "ES2022 private in (`#{{private}} in {{object}}`) is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "BinaryExpression[operator='in'] > PrivateIdentifier.left"(node) {
                context.report({
                    node,
                    messageId: "forbidden",
                    data: {
                        private: node.name,
                        object:
                            node.parent.right.type === "Identifier"
                                ? node.parent.right.name
                                : "object",
                    },
                })
            },
        }
    },
}
