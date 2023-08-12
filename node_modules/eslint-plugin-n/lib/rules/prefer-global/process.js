/**
 * @author Toru Nagashima
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { READ } = require("@eslint-community/eslint-utils")
const checkForPreferGlobal = require("../../util/check-prefer-global")

const trackMap = {
    globals: {
        process: { [READ]: true },
    },
    modules: {
        process: { [READ]: true },
    },
}

module.exports = {
    meta: {
        docs: {
            description: 'enforce either `process` or `require("process")`',
            category: "Stylistic Issues",
            recommended: false,
            url: "https://github.com/weiran-zsd/eslint-plugin-node/blob/HEAD/docs/rules/prefer-global/process.md",
        },
        type: "suggestion",
        fixable: null,
        schema: [{ enum: ["always", "never"] }],
        messages: {
            preferGlobal:
                "Unexpected use of 'require(\"process\")'. Use the global variable 'process' instead.",
            preferModule:
                "Unexpected use of the global variable 'process'. Use 'require(\"process\")' instead.",
        },
    },

    create(context) {
        return {
            "Program:exit"() {
                checkForPreferGlobal(context, trackMap)
            },
        }
    },
}
