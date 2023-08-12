/**
 * @author Toru Nagashima <https://github.com/mysticatea>
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { READ, ReferenceTracker } = require("@eslint-community/eslint-utils")

module.exports = {
    meta: {
        docs: {
            description: "disallow the `Atomics` class.",
            category: "ES2017",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-atomics.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2017 '{{name}}' class is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "Program:exit"() {
                const tracker = new ReferenceTracker(context.getScope())
                for (const { node, path } of tracker.iterateGlobalReferences({
                    Atomics: { [READ]: true },
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
