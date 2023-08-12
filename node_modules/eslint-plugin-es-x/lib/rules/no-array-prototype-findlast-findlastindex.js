"use strict"

const {
    definePrototypeMethodHandler,
} = require("../util/define-prototype-method-handler")

module.exports = {
    meta: {
        docs: {
            description:
                "disallow the `Array.prototype.{findLast,findLastIndex}` methods.",
            category: "ES2023",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-array-prototype-findlast-findlastindex.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2023 '{{name}}' method is forbidden.",
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
            Array: ["findLast", "findLastIndex"],
            Int8Array: ["findLast", "findLastIndex"],
            Uint8Array: ["findLast", "findLastIndex"],
            Uint8ClampedArray: ["findLast", "findLastIndex"],
            Int16Array: ["findLast", "findLastIndex"],
            Uint16Array: ["findLast", "findLastIndex"],
            Int32Array: ["findLast", "findLastIndex"],
            Uint32Array: ["findLast", "findLastIndex"],
            Float32Array: ["findLast", "findLastIndex"],
            Float64Array: ["findLast", "findLastIndex"],
            BigInt64Array: ["findLast", "findLastIndex"],
            BigUint64Array: ["findLast", "findLastIndex"],
        })
    },
}
