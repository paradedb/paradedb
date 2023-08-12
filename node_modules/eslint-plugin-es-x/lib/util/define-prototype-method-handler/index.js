"use strict"

const { getPropertyName } = require("@eslint-community/eslint-utils")
const { buildObjectTypeChecker } = require("./object-type-checker")
const { buildObjectTypeCheckerForTS } = require("./object-type-checker-for-ts")

/**
 * @typedef {object} CreateReportArgument
 * @property {true | 'aggressive'} objectTypeResult
 * @property {string} propertyName
 * @property {MemberExpression} node
 */
/**
 * @typedef {object} Options
 * @property { (arg: CreateReportArgument) => ReportDescriptor } [Options.createReport]
 */

/**
 * Define handlers to disallow prototype methods.
 * @param {RuleContext} context The rule context.
 * @param {Record<string, readonly string[]>} nameMap The method names to disallow. The key is class names and that value is method names.
 * @param {Options} [options] The options.
 * @returns {Record<string, (node: ASTNode) => void>} The defined handlers.
 */
function definePrototypeMethodHandler(context, nameMap, options) {
    const aggressiveOption = getAggressiveOption(context)
    const aggressiveResult = aggressiveOption ? "aggressive" : false

    const objectTypeChecker =
        buildObjectTypeCheckerForTS(context, aggressiveResult) ||
        buildObjectTypeChecker(context, aggressiveResult)

    /**
     * Check if the type of the given node is one of given class or not.
     * @param {MemberExpression} memberAccessNode The MemberExpression node.
     * @param {string} className The class name to disallow.
     * @returns {boolean | "aggressive"} `true` if should disallow it.
     */
    function checkObjectType(memberAccessNode, className) {
        // If it's obvious, shortcut.
        if (memberAccessNode.object.type === "ArrayExpression") {
            return className === "Array"
        }
        if (
            memberAccessNode.object.type === "Literal" &&
            memberAccessNode.object.regex
        ) {
            return className === "RegExp"
        }
        if (
            (memberAccessNode.object.type === "Literal" &&
                typeof memberAccessNode.object.value === "string") ||
            memberAccessNode.object.type === "TemplateLiteral"
        ) {
            return className === "String"
        }
        if (
            memberAccessNode.object.type === "FunctionExpression" ||
            memberAccessNode.object.type === "ArrowFunctionExpression"
        ) {
            return className === "Function"
        }

        // Test object type.
        return objectTypeChecker(memberAccessNode, className)
    }

    // For performance
    const nameMapEntries = Object.entries(nameMap)
    if (nameMapEntries.length === 1) {
        const [[className, methodNames]] = nameMapEntries
        return {
            MemberExpression(node) {
                const propertyName = getPropertyName(node, context.getScope())
                let objectTypeResult = undefined
                if (
                    methodNames.includes(propertyName) &&
                    (objectTypeResult = checkObjectType(node, className))
                ) {
                    context.report({
                        node,
                        messageId: "forbidden",
                        data: {
                            name: `${className}.prototype.${propertyName}`,
                        },
                        ...((options &&
                            options.createReport &&
                            options.createReport({
                                objectTypeResult,
                                propertyName,
                                node,
                            })) ||
                            {}),
                    })
                }
            },
        }
    }

    return {
        MemberExpression(node) {
            const propertyName = getPropertyName(node, context.getScope())
            for (const [className, methodNames] of nameMapEntries) {
                if (
                    methodNames.includes(propertyName) &&
                    checkObjectType(node, className)
                ) {
                    context.report({
                        node,
                        messageId: "forbidden",
                        data: {
                            name: `${className}.prototype.${propertyName}`,
                        },
                    })
                    return
                }
            }
        },
    }
}

/**
 * Get `aggressive` option value.
 * @param {RuleContext} context The rule context.
 * @returns {boolean} The gotten `aggressive` option value.
 */
function getAggressiveOption(context) {
    const options = context.options[0]
    const globalOptions = context.settings["es-x"]

    if (options && typeof options.aggressive === "boolean") {
        return options.aggressive
    }
    if (globalOptions && typeof globalOptions.aggressive === "boolean") {
        return globalOptions.aggressive
    }

    return false
}

module.exports = { definePrototypeMethodHandler }
