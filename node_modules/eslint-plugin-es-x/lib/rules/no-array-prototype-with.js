"use strict"

const {
    definePrototypeMethodHandler,
} = require("../util/define-prototype-method-handler")

module.exports = {
    meta: {
        docs: {
            description: "disallow the `Array.prototype.with` method.",
            category: "ES2023",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-array-prototype-with.html",
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
            Array: ["with"],
            Int8Array: ["with"],
            Uint8Array: ["with"],
            Uint8ClampedArray: ["with"],
            Int16Array: ["with"],
            Uint16Array: ["with"],
            Int32Array: ["with"],
            Uint32Array: ["with"],
            Float32Array: ["with"],
            Float64Array: ["with"],
            BigInt64Array: ["with"],
            BigUint64Array: ["with"],
        })
    },
}
