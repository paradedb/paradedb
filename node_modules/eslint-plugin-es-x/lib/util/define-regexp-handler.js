"use strict"

const { RegExpValidator } = require("@eslint-community/regexpp")
const { getRegExpCalls } = require("../utils")

const allVisitorBuilder = new WeakMap()

/**
 * @typedef {RegExpValidator.Options & {onExit:()=>void}} RuleValidator
 */
/**
 * Define handlers to regexp nodes.
 * @param {RuleContext} context The rule context.
 * @param {(node: Node) => RuleValidator} visitorBuilder The regexp node visitor builder.
 * @returns {Record<string, (node: ASTNode) => void>} The defined handlers.
 */
function defineRegExpHandler(context, visitorBuilder) {
    const programNode = context.getSourceCode().ast

    let handler = {}
    let builders = allVisitorBuilder.get(programNode)
    if (!builders) {
        builders = []
        allVisitorBuilder.set(programNode, builders)
        handler = {
            "Literal[regex]"(node) {
                const { pattern, flags } = node.regex
                visitRegExp(builders, node, pattern || "", flags || "")
            },

            "Program:exit"() {
                allVisitorBuilder.delete(programNode)

                const scope = context.getScope()
                for (const { node, pattern, flags } of getRegExpCalls(scope)) {
                    visitRegExp(builders, node, pattern || "", flags || "")
                }
            },
        }
    }

    builders.push(visitorBuilder)

    return handler
}

module.exports = { defineRegExpHandler }

/**
 * Visit a given regular expression.
 * @param {((node: Node) => RuleValidator)[]} visitorBuilders The array of validator options builders.
 * @param {Node} node The AST node to report.
 * @param {string} pattern The pattern part of a RegExp.
 * @param {string} flags The flags part of a RegExp.
 * @returns {void}
 */
function visitRegExp(visitorBuilders, node, pattern, flags) {
    try {
        const visitors = visitorBuilders.map((r) => r(node, { pattern, flags }))
        const composedVisitor = composeRegExpVisitors(visitors)

        new RegExpValidator(composedVisitor).validatePattern(
            pattern,
            0,
            pattern.length,
            flags.includes("u"),
        )

        if (typeof composedVisitor.onExit === "function") {
            composedVisitor.onExit()
        }
    } catch (error) {
        //istanbul ignore else
        if (error.message.startsWith("Invalid regular expression:")) {
            return
        }
        //istanbul ignore next
        throw error
    }
}

/**
 * Returns a single visitor handler that executes all the given visitors.
 * @param {RuleValidator[]} visitors
 * @returns {RuleValidator}
 */
function composeRegExpVisitors(visitors) {
    const result = {}

    for (const visitor of visitors) {
        const entries = Object.entries(visitor)

        for (const [key, fn] of entries) {
            const orig = result[key]
            if (orig) {
                result[key] = (...args) => {
                    orig(...args)
                    fn(...args)
                }
            } else {
                result[key] = fn
            }
        }
    }

    return result
}
