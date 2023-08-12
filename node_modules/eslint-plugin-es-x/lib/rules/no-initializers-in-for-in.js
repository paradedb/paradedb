"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow initializers in for-in heads.",
            category: "legacy",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-initializers-in-for-in.html",
        },
        fixable: null,
        messages: {
            forbidden:
                "Annex B feature the initializers in for-in heads are forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "ForInStatement > VariableDeclaration.left > VariableDeclarator.declarations > *.init"(
                node,
            ) {
                context.report({ node, messageId: "forbidden" })
            },
        }
    },
}
