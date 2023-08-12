"use strict"

const {
    definePrototypeMethodHandler,
} = require("../util/define-prototype-method-handler")

module.exports = {
    meta: {
        docs: {
            description: "disallow the `Array.prototype.toReversed` method.",
            category: "ES2023",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-array-prototype-toreversed.html",
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
            Array: ["toReversed"],
            Int8Array: ["toReversed"],
            Uint8Array: ["toReversed"],
            Uint8ClampedArray: ["toReversed"],
            Int16Array: ["toReversed"],
            Uint16Array: ["toReversed"],
            Int32Array: ["toReversed"],
            Uint32Array: ["toReversed"],
            Float32Array: ["toReversed"],
            Float64Array: ["toReversed"],
            BigInt64Array: ["toReversed"],
            BigUint64Array: ["toReversed"],
        })
    },
}
