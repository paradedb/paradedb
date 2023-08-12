/**
 * @author Yosuke Ota <https://github.com/ota-meshi>
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
                "disallow the `{Array,String}.prototype.at()` methods.",
            category: "ES2022",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-array-string-prototype-at.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2022 '{{name}}' method is forbidden.",
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
            Array: ["at"],
            String: ["at"],
            Int8Array: ["at"],
            Uint8Array: ["at"],
            Uint8ClampedArray: ["at"],
            Int16Array: ["at"],
            Uint16Array: ["at"],
            Int32Array: ["at"],
            Uint32Array: ["at"],
            Float32Array: ["at"],
            Float64Array: ["at"],
            BigInt64Array: ["at"],
            BigUint64Array: ["at"],
        })
    },
}
