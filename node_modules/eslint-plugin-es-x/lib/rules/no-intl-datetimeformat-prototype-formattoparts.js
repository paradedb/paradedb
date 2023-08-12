"use strict"

const {
    definePrototypeMethodHandler,
} = require("../util/define-prototype-method-handler")

module.exports = {
    meta: {
        docs: {
            description:
                "disallow the `DateTimeFormat.prototype.formatToParts` method.",
            category: "ES2017-Intl-API",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-intl-datetimeformat-prototype-formattoparts.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2017 Intl API '{{name}}' method is forbidden.",
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
            "Intl.DateTimeFormat": ["formatToParts"],
        })
    },
}
