"use strict"

const {
    definePrototypeMethodHandler,
} = require("../util/define-prototype-method-handler")

module.exports = {
    meta: {
        docs: {
            description:
                "disallow the `NumberFormat.prototype.formatToParts` method.",
            category: "ES2018-Intl-API",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-intl-numberformat-prototype-formattoparts.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2018 Intl API '{{name}}' method is forbidden.",
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
            "Intl.NumberFormat": ["formatToParts"],
        })
    },
}
