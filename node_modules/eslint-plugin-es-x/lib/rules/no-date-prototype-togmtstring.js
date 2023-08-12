"use strict"

const {
    definePrototypeMethodHandler,
} = require("../util/define-prototype-method-handler")

module.exports = {
    meta: {
        docs: {
            description: "disallow the `Date.prototype.toGMTString` method.",
            category: "legacy",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-date-prototype-togmtstring.html",
        },
        fixable: "code",
        messages: {
            forbidden: "Annex B feature '{{name}}' method is forbidden.",
        },
        schema: [
            {
                type: "object",
                properties: {
                    aggressive: { type: "boolean" },
                },
                additionalProperties: false,
            },
        ],
        type: "problem",
        hasSuggestions: true,
    },
    create(context) {
        return definePrototypeMethodHandler(
            context,
            {
                Date: ["toGMTString"],
            },
            {
                createReport({ objectTypeResult, node }) {
                    if (node.computed) {
                        return null
                    }
                    if (objectTypeResult !== true) {
                        return {
                            suggest: [
                                {
                                    desc: "Replace with 'toUTCString'",
                                    fix,
                                },
                            ],
                        }
                    }
                    return {
                        fix,
                    }

                    function fix(fixer) {
                        return fixer.replaceText(node.property, "toUTCString")
                    }
                },
            },
        )
    },
}
