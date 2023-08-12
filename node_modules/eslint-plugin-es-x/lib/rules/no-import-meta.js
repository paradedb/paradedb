/**
 * @author Yosuke Ota <https://github.com/ota-meshi>
 * See LICENSE file in root directory for full license.
 */
"use strict"

module.exports = {
    meta: {
        docs: {
            description: "disallow `import.meta` meta property.",
            category: "ES2020",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-import-meta.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2020 'import.meta' meta property is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "MetaProperty[meta.name='import'][property.name='meta']"(node) {
                context.report({ node, messageId: "forbidden" })
            },
        }
    },
}
