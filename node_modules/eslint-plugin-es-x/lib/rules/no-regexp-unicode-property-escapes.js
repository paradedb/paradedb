/**
 * @author Toru Nagashima <https://github.com/mysticatea>
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { defineRegExpHandler } = require("../util/define-regexp-handler")

module.exports = {
    meta: {
        docs: {
            description: "disallow RegExp Unicode property escape sequences.",
            category: "ES2018",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-regexp-unicode-property-escapes.html",
        },
        fixable: null,
        messages: {
            forbidden:
                "ES2018 RegExp Unicode property escape sequences are forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return defineRegExpHandler(context, (node) => {
            let found = false
            return {
                onUnicodePropertyCharacterSet() {
                    found = true
                },
                onExit() {
                    if (found) {
                        context.report({ node, messageId: "forbidden" })
                    }
                },
            }
        })
    },
}
