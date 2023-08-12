/**
 * @author Toru Nagashima <https://github.com/mysticatea>
 * See LICENSE file in root directory for full license.
 */
"use strict"

const {
    definePrototypeMethodHandler,
} = require("../util/define-prototype-method-handler")

module.exports = {
    meta: {
        docs: {
            description:
                "disallow the `String.prototype.{padStart,padEnd}` methods.",
            category: "ES2017",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-string-prototype-padstart-padend.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2017 '{{name}}' method is forbidden.",
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
            String: ["padEnd", "padStart"],
        })
    },
}
