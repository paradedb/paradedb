"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow Hashbang comments.",
            category: "ES2023",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-hashbang.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2023 Hashbang comments are forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            Program() {
                const firstComment = context.getSourceCode().ast.comments[0]
                if (firstComment && firstComment.type === "Shebang") {
                    context.report({
                        node: firstComment,
                        messageId: "forbidden",
                    })
                }
            },
        }
    },
}
