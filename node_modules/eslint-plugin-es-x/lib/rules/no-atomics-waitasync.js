"use strict"

const { READ, ReferenceTracker } = require("@eslint-community/eslint-utils")

module.exports = {
    meta: {
        docs: {
            description: "disallow the `Atomics.waitAsync` method.",
            category: "ES2024",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-atomics-waitasync.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2024 '{{name}}' method is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "Program:exit"() {
                const tracker = new ReferenceTracker(context.getScope())
                for (const { node, path } of tracker.iterateGlobalReferences({
                    Atomics: {
                        waitAsync: { [READ]: true },
                    },
                })) {
                    context.report({
                        node,
                        messageId: "forbidden",
                        data: { name: path.join(".") },
                    })
                }
            },
        }
    },
}
