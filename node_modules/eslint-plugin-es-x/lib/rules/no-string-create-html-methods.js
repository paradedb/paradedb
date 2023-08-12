"use strict"

const {
    definePrototypeMethodHandler,
} = require("../util/define-prototype-method-handler")

module.exports = {
    meta: {
        docs: {
            description: "disallow HTML creation methods of string instances.",
            category: "legacy",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-string-create-html-methods.html",
        },
        fixable: null,
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
    },
    create(context) {
        return definePrototypeMethodHandler(context, {
            String: [
                "anchor",
                "big",
                "blink",
                "bold",
                "fixed",
                "fontcolor",
                "fontsize",
                "italics",
                "link",
                "small",
                "strike",
                "sub",
                "sup",
            ],
        })
    },
}
