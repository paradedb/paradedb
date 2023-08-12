/**
 * @author Toru Nagashima <https://github.com/mysticatea>
 * See LICENSE file in root directory for full license.
 */
"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow optional `catch` binding.",
            category: "ES2019",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-optional-catch-binding.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2019 optional 'catch' binding is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "CatchClause[param=null]"(node) {
                context.report({ node, messageId: "forbidden" })
            },
        }
    },
}
