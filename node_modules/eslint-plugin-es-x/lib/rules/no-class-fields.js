/**
 * @author Yosuke Ota <https://github.com/ota-meshi>
 * See LICENSE file in root directory for full license.
 */
"use strict"

const {
    getFunctionNameWithKind,
    getPropertyName,
} = require("@eslint-community/eslint-utils")

/**
 * Get the name and kind of the given PropertyDefinition node.
 * @param {PropertyDefinition} node - The PropertyDefinition node to get.
 * @param {SourceCode} sourceCode The source code object to get the code of computed property keys.
 * @returns {string} The name and kind of the PropertyDefinition node.
 */
function getFieldNameWithKind(node, sourceCode) {
    const tokens = []

    if (node.static) {
        tokens.push("static")
    }
    if (node.key.type === "PrivateIdentifier") {
        tokens.push("private")
    }

    tokens.push("field")

    if (node.key.type === "PrivateIdentifier") {
        tokens.push(`#${node.key.name}`)
    } else {
        const name = getPropertyName(node)
        if (name) {
            tokens.push(`'${name}'`)
        } else if (sourceCode) {
            const keyText = sourceCode.getText(node.key)
            if (!keyText.includes("\n")) {
                tokens.push(`[${keyText}]`)
            }
        }
    }

    return tokens.join(" ")
}

module.exports = {
    meta: {
        docs: {
            description: "disallow class fields.",
            category: "ES2022",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-class-fields.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2022 {{nameWithKind}} is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            PropertyDefinition(node) {
                context.report({
                    node: node.key,
                    messageId: "forbidden",
                    data: {
                        nameWithKind: getFieldNameWithKind(
                            node,
                            context.getSourceCode(),
                        ),
                    },
                })
            },
            "MethodDefinition:exit"(node) {
                if (node.key.type !== "PrivateIdentifier") {
                    return
                }
                context.report({
                    node: node.key,
                    messageId: "forbidden",
                    data: {
                        nameWithKind: getFunctionNameWithKind(
                            node.value,
                            context.getSourceCode(),
                        ),
                    },
                })
            },
            ":not(PropertyDefinition, MethodDefinition) > PrivateIdentifier"(
                node,
            ) {
                const parent = node.parent
                context.report({
                    node,
                    messageId: "forbidden",
                    data: {
                        nameWithKind:
                            parent.parent.type === "CallExpression" &&
                            parent.parent.callee === parent
                                ? `private method call #${node.name}()`
                                : `private access #${node.name}`,
                    },
                })
            },
        }
    },
}
