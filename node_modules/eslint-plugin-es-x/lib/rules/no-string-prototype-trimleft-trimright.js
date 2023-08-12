"use strict"

const {
    definePrototypeMethodHandler,
} = require("../util/define-prototype-method-handler")

module.exports = {
    meta: {
        docs: {
            description:
                "disallow the `String.prototype.{trimLeft,trimRight}` methods.",
            category: "legacy",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-string-prototype-trimleft-trimright.html",
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
                String: ["trimLeft", "trimRight"],
            },
            {
                createReport({ objectTypeResult, node, propertyName }) {
                    if (node.computed) {
                        return null
                    }
                    const newPropertyName =
                        propertyName === "trimLeft" ? "trimStart" : "trimEnd"
                    if (objectTypeResult !== true) {
                        return {
                            suggest: [
                                {
                                    desc: `Replace with '${newPropertyName}'`,
                                    fix,
                                },
                            ],
                        }
                    }
                    return {
                        fix,
                    }

                    function fix(fixer) {
                        return fixer.replaceText(node.property, newPropertyName)
                    }
                },
            },
        )
    },
}
