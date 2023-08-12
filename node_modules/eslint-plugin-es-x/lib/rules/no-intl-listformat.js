"use strict"

const { READ, ReferenceTracker } = require("@eslint-community/eslint-utils")

module.exports = {
    meta: {
        docs: {
            description: "disallow the `Intl.ListFormat` object.",
            category: "ES2021-Intl-API",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-intl-listformat.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2021 Intl API '{{name}}' object is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "Program:exit"() {
                const tracker = new ReferenceTracker(context.getScope())
                for (const { node, path } of tracker.iterateGlobalReferences({
                    Intl: {
                        ListFormat: { [READ]: true },
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
