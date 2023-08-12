/**
 * @author Yosuke Ota
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { getRegExpCalls } = require("../utils")

module.exports = {
    meta: {
        docs: {
            description: "disallow RegExp `d` flag.",
            category: "ES2022",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-regexp-d-flag.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2022 RegExp 'd' flag is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "Literal[regex]"(node) {
                if (node.regex.flags.includes("d")) {
                    context.report({ node, messageId: "forbidden" })
                }
            },

            "Program:exit"() {
                const scope = context.getScope()

                for (const { node, flags } of getRegExpCalls(scope)) {
                    if (flags && flags.includes("d")) {
                        context.report({ node, messageId: "forbidden" })
                    }
                }
            },
        }
    },
}
