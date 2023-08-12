"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow labelled function declarations.",
            category: "legacy",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-labelled-function-declarations.html",
        },
        fixable: null,
        messages: {
            forbidden:
                "Annex B feature the labelled function declarations are forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "LabeledStatement > FunctionDeclaration.body"(node) {
                context.report({ node: node.parent, messageId: "forbidden" })
            },
        }
    },
}
