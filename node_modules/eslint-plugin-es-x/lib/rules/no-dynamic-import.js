/**
 * @author Toru Nagashima <https://github.com/mysticatea>
 * See LICENSE file in root directory for full license.
 */
"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow `import()` syntax",
            category: "ES2020",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-dynamic-import.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2020 'import()' syntax is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            ImportExpression(node) {
                context.report({ messageId: "forbidden", node })
            },
        }
    },
}
