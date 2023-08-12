"use strict"

const {
    getPropertyName,
    findVariable,
} = require("@eslint-community/eslint-utils")

const LEGACY_ACCESSOR_METHODS = new Set([
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
])

module.exports = {
    meta: {
        docs: {
            description: "disallow legacy `Object.prototype` accessor methods.",
            category: "legacy",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-legacy-object-prototype-accessor-methods.html",
        },
        fixable: null,
        messages: {
            forbidden: "LEGACY '{{name}}' method is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        const reported = new Set()

        function report(name, node) {
            if (reported.has(node)) {
                return
            }
            reported.add(node)
            context.report({
                node,
                messageId: "forbidden",
                data: {
                    name,
                },
            })
        }

        return {
            MemberExpression(node) {
                const name = getPropertyName(node)
                if (!LEGACY_ACCESSOR_METHODS.has(name)) {
                    return
                }
                report(name, node.property)
            },
            Identifier(node) {
                const name = node.name
                if (!LEGACY_ACCESSOR_METHODS.has(name)) {
                    return
                }
                if (
                    node.parent.type === "MemberExpression" &&
                    node.parent.property === node
                ) {
                    // Already reported.
                    return
                }
                if (
                    node.parent.type === "Property" &&
                    !node.parent.shorthand &&
                    node.parent.key === node
                ) {
                    return
                }
                const scopeManager = context.getSourceCode().scopeManager
                if (
                    // Not defined as global variables.
                    !scopeManager.globalScope.through.some(
                        ({ identifier }) => identifier === node,
                    )
                ) {
                    const variable = findVariable(context.getScope(), node)
                    if (!variable) {
                        return
                    }
                    // It is defined as global variables.
                    if (variable.defs.length) {
                        return
                    }
                }
                report(name, node)
            },
        }
    },
}
