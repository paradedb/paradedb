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
            description: "disallow the `RegExp.prototype.flags` property.",
            category: "ES2015",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-regexp-prototype-flags.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2015 '{{name}}' property is forbidden.",
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
            RegExp: ["flags"],
        })
    },
}
