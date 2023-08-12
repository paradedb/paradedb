"use strict"

const { READ, ReferenceTracker } = require("@eslint-community/eslint-utils")

module.exports = {
    meta: {
        docs: {
            description: "disallow `escape` and `unescape`",
            category: "legacy",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-escape-unescape.html",
        },
        fixable: null,
        messages: {
            forbidden: "Annex B feature '{{name}}' is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "Program:exit"() {
                const tracker = new ReferenceTracker(context.getScope())
                for (const { node, path } of tracker.iterateGlobalReferences({
                    escape: { [READ]: true },
                    unescape: { [READ]: true },
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
