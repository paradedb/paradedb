/**
 * @author Yosuke Ota
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { READ, ReferenceTracker } = require("@eslint-community/eslint-utils")

module.exports = {
    meta: {
        docs: {
            description:
                "disallow the `WeakRef` and `FinalizationRegistry` class.",
            category: "ES2021",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-weakrefs.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2021 '{{name}}' class is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "Program:exit"() {
                const tracker = new ReferenceTracker(context.getScope())
                for (const { node, path } of tracker.iterateGlobalReferences({
                    FinalizationRegistry: { [READ]: true },
                    WeakRef: { [READ]: true },
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
