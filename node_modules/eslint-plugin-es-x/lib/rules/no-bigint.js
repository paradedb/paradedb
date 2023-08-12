/**
 * @author Toru Nagashima <https://github.com/mysticatea>
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { ReferenceTracker } = require("@eslint-community/eslint-utils")

module.exports = {
    meta: {
        docs: {
            description: "disallow `bigint` syntax and built-ins",
            category: "ES2020",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-bigint.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2020 BigInt is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            Literal(node) {
                if (node.bigint != null) {
                    context.report({ messageId: "forbidden", node })
                }
            },

            "Program:exit"() {
                const tracker = new ReferenceTracker(context.getScope())
                const references = tracker.iterateGlobalReferences({
                    BigInt: { [ReferenceTracker.READ]: true },
                    BigInt64Array: { [ReferenceTracker.READ]: true },
                    BigUint64Array: { [ReferenceTracker.READ]: true },
                })

                for (const { node } of references) {
                    context.report({ messageId: "forbidden", node })
                }
            },
        }
    },
}
