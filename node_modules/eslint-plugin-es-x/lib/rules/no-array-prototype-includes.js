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
            description: "disallow the `Array.prototype.includes` method.",
            category: "ES2016",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-array-prototype-includes.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2016 '{{name}}' method is forbidden.",
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
            Array: ["includes"],
            Int8Array: ["includes"],
            Uint8Array: ["includes"],
            Uint8ClampedArray: ["includes"],
            Int16Array: ["includes"],
            Uint16Array: ["includes"],
            Int32Array: ["includes"],
            Uint32Array: ["includes"],
            Float32Array: ["includes"],
            Float64Array: ["includes"],
            BigInt64Array: ["includes"],
            BigUint64Array: ["includes"],
        })
    },
}
