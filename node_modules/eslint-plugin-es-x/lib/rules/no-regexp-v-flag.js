"use strict"

const { getRegExpCalls } = require("../utils")

module.exports = {
    meta: {
        docs: {
            description: "disallow RegExp `v` flag.",
            category: "ES2024",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-regexp-v-flag.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2024 RegExp 'v' flag is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "Literal[regex]"(node) {
                if (node.regex.flags.includes("v")) {
                    context.report({ node, messageId: "forbidden" })
                }
            },

            "Program:exit"() {
                const scope = context.getScope()

                for (const { node, flags } of getRegExpCalls(scope)) {
                    if (flags && flags.includes("v")) {
                        context.report({ node, messageId: "forbidden" })
                    }
                }
            },
        }
    },
}
