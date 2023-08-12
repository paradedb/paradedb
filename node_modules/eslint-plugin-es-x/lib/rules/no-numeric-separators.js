/**
 * @author Yosuke Ota
 * See LICENSE file in root directory for full license.
 */
"use strict"

/**
 * Remove the numeric separators.
 * @param  {string} raw The raw string of numeric literals
 * @returns {string} The string with the separators removed.
 */
function removeNumericSeparators(raw) {
    return raw.replace(/_/gu, "")
}

module.exports = {
    meta: {
        docs: {
            description: "disallow numeric separators.",
            category: "ES2021",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-numeric-separators.html",
        },
        fixable: "code",
        messages: {
            forbidden: "ES2021 numeric separators are forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            Literal(node) {
                if (
                    (typeof node.value === "number" || node.bigint != null) &&
                    node.raw.includes("_")
                ) {
                    context.report({
                        node,
                        messageId: "forbidden",
                        fix: (fixer) =>
                            fixer.replaceText(
                                node,
                                removeNumericSeparators(node.raw),
                            ),
                    })
                }
            },
        }
    },
}
