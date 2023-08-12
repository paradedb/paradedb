"use strict"

module.exports = {
    meta: {
        docs: {
            description:
                "disallow function declarations in if statement clauses without using blocks.",
            category: "legacy",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-function-declarations-in-if-statement-clauses-without-block.html",
        },
        fixable: "code",
        messages: {
            forbidden:
                "Annex B feature the function declarations in if statement clauses without using blocks are forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "IfStatement > FunctionDeclaration.consequent, IfStatement > FunctionDeclaration.alternate"(
                node,
            ) {
                context.report({
                    node,
                    messageId: "forbidden",
                    fix(fixer) {
                        return [
                            fixer.insertTextBefore(node, "{"),
                            fixer.insertTextAfter(node, "}"),
                        ]
                    },
                })
            },
        }
    },
}
