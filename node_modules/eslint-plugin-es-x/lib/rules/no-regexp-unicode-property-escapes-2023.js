/**
 * @author Yosuke Ota
 * See LICENSE file in root directory for full license.
 */
"use strict"

const { defineRegExpHandler } = require("../util/define-regexp-handler")
const {
    scNameSet,
    scValueSets,
    binPropertySets,
} = require("../util/unicode-properties")

function isNewUnicodePropertyKeyValuePair(key, value) {
    return scNameSet.has(key) && scValueSets.es2023.has(value)
}

function isNewBinaryUnicodeProperty(key) {
    return binPropertySets.es2023.has(key)
}

module.exports = {
    meta: {
        docs: {
            description:
                "disallow the new values of RegExp Unicode property escape sequences in ES2023",
            category: "ES2023",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-regexp-unicode-property-escapes-2023.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2023 '{{value}}' is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return defineRegExpHandler(context, (node, { pattern }) => {
            let foundValue = ""
            return {
                onUnicodePropertyCharacterSet(start, end, _kind, key, value) {
                    if (foundValue) {
                        return
                    }
                    if (
                        value
                            ? isNewUnicodePropertyKeyValuePair(key, value)
                            : isNewBinaryUnicodeProperty(key)
                    ) {
                        foundValue = pattern.slice(start, end)
                    }
                },
                onExit() {
                    if (foundValue) {
                        context.report({
                            node,
                            messageId: "forbidden",
                            data: { value: foundValue },
                        })
                    }
                },
            }
        })
    },
}
