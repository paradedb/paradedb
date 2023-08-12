/**
 * @author Sosuke Suzuki <https://github.com/sosukesuzuki>
 * See LICENSE file in root directory for full license.
 */

"use strict"

const {
    CONSTRUCT,
    READ,
    ReferenceTracker,
    getPropertyName,
} = require("@eslint-community/eslint-utils")

/**
 * @typedef {import("estree").Node} Node
 * @typedef {import("estree").ClassExpression | import("estree").ClassDeclaration} ClassNode
 * @typedef {import("estree").CallExpression} CallExpression
 */

const errorConstructorNames = [
    "Error",
    "EvalError",
    "RangeError",
    "ReferenceError",
    "SyntaxError",
    "TypeError",
    "URIError",
    "AggregateError",
]
const errorsTraceMap = {}
for (const errorConstructorName of errorConstructorNames) {
    errorsTraceMap[errorConstructorName] = { [CONSTRUCT]: true, [READ]: true }
}

/**
 * @param {Node} node
 * @returns {boolean}
 */
function isSuperCall(node) {
    return node.type === "CallExpression" && node.callee.type === "Super"
}

/**
 * @param {Node|undefined} node
 * @returns {boolean}
 */
function isSpreadElement(node) {
    return node && node.type === "SpreadElement"
}

/**
 * @param {Node} node
 * @returns {ClassNode | null}
 */
function findClassFromAncestors(node) {
    if (node.type !== "ClassExpression" && node.type !== "ClassDeclaration") {
        return findClassFromAncestors(node.parent)
    }
    if (!node) {
        return null
    }
    return node
}

module.exports = {
    meta: {
        docs: {
            description: "disallow Error Cause.",
            category: "ES2022",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-error-cause.html",
        },
        fixable: null,
        messages: {
            forbidden: "ES2022 Error Cause is forbidden.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        /** @type {Array<{ classNode: ClassNode, superCallNode: CallExpression }>} */
        const maybeErrorSubclasses = []

        /** @type {Array<{ classNode: ClassNode, superCallNode: CallExpression }>} */
        const maybeAggregateErrorSubclasses = []

        /**
         * Checks if the received node is a constructor call with cause option.
         * e.g. `new Error("message", { cause: foo })`, `super("message", { cause: foo })`
         *
         * @param {Node} node
         * @param {boolean} isAggregateError
         * @returns {boolean}
         */
        function isConstructCallWithCauseOption(node, isAggregateError) {
            if (node.type !== "NewExpression" && !isSuperCall(node)) {
                return false
            }
            const optionsArgIndex = isAggregateError ? 2 : 1
            for (let index = 0; index < optionsArgIndex; index++) {
                if (isSpreadElement(node.arguments[index])) {
                    return false
                }
            }
            const optionsArg = node.arguments[optionsArgIndex]
            if (!optionsArg || optionsArg.type !== "ObjectExpression") {
                return false
            }
            return optionsArg.properties.some((property) => {
                if (property.type !== "Property") {
                    return false
                }
                // new Error("msg", { cause: foo })
                return getPropertyName(property, context.getScope()) === "cause"
            })
        }

        /**
         * @param {Node} node
         * @param {isAggregateError} boolean
         * @return {Node | null}
         */
        function getReportedNode(node, isAggregateError) {
            const errorSubclasses = isAggregateError
                ? maybeAggregateErrorSubclasses
                : maybeErrorSubclasses

            if (errorSubclasses.length > 0) {
                for (const { classNode, superCallNode } of errorSubclasses) {
                    if (classNode.superClass === node) {
                        return superCallNode
                    }
                }
            }
            if (isConstructCallWithCauseOption(node, isAggregateError)) {
                return node
            }
            return null
        }

        return {
            Super(node) {
                const superCallNode = node.parent

                function findErrorSubclasses(isAggregateError) {
                    const errorSubclasses = isAggregateError
                        ? maybeAggregateErrorSubclasses
                        : maybeErrorSubclasses

                    if (
                        isConstructCallWithCauseOption(
                            superCallNode,
                            isAggregateError,
                        )
                    ) {
                        const classNode = findClassFromAncestors(superCallNode)
                        if (classNode && classNode.superClass) {
                            errorSubclasses.push({ classNode, superCallNode })
                        }
                    }
                }

                findErrorSubclasses(/* isAggregateError */ false)
                findErrorSubclasses(/* isAggregateError */ true)
            },
            "Program:exit"() {
                const tracker = new ReferenceTracker(context.getScope())
                for (const { node, path } of tracker.iterateGlobalReferences(
                    errorsTraceMap,
                )) {
                    const reportedNode = getReportedNode(
                        node,
                        path.join(",") === "AggregateError",
                    )
                    if (reportedNode) {
                        context.report({
                            node: reportedNode,
                            messageId: "forbidden",
                        })
                    }
                }
            },
        }
    },
}
