/**
 * @author Toru Nagashima
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { checkForRestriction, messages } = require("../util/check-restricted")
const visit = require("../util/visit-import")

module.exports = {
    meta: {
        type: "suggestion",
        docs: {
            description:
                "disallow specified modules when loaded by `import` declarations",
            category: "Stylistic Issues",
            recommended: false,
            url: "https://github.com/weiran-zsd/eslint-plugin-node/blob/HEAD/docs/rules/no-restricted-import.md",
        },
        fixable: null,
        schema: [
            {
                type: "array",
                items: {
                    anyOf: [
                        { type: "string" },
                        {
                            type: "object",
                            properties: {
                                name: {
                                    anyOf: [
                                        { type: "string" },
                                        {
                                            type: "array",
                                            items: { type: "string" },
                                            additionalItems: false,
                                        },
                                    ],
                                },
                                message: { type: "string" },
                            },
                            additionalProperties: false,
                            required: ["name"],
                        },
                    ],
                },
                additionalItems: false,
            },
        ],
        messages,
    },

    create(context) {
        const opts = { includeCore: true }
        return visit(context, opts, targets =>
            checkForRestriction(context, targets)
        )
    },
}
