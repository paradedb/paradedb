"use strict"

const {
    definePrototypeMethodHandler,
} = require("../util/define-prototype-method-handler")

module.exports = {
    meta: {
        docs: {
            description: "disallow the `Array.prototype.toSorted` method.",
            category: "ES2023",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-array-prototype-tosorted.html",
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
            Array: ["toSorted"],
            Int8Array: ["toSorted"],
            Uint8Array: ["toSorted"],
            Uint8ClampedArray: ["toSorted"],
            Int16Array: ["toSorted"],
            Uint16Array: ["toSorted"],
            Int32Array: ["toSorted"],
            Uint32Array: ["toSorted"],
            Float32Array: ["toSorted"],
            Float64Array: ["toSorted"],
            BigInt64Array: ["toSorted"],
            BigUint64Array: ["toSorted"],
        })
    },
}
