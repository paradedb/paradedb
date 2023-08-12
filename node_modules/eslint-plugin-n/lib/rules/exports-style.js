/**
 * @author Toru Nagashima
 * See LICENSE file in root directory for full license.
 */
"use strict"

/*istanbul ignore next */
/**
 * This function is copied from https://github.com/eslint/eslint/blob/2355f8d0de1d6732605420d15ddd4f1eee3c37b6/lib/ast-utils.js#L648-L684
 *
 * @param {ASTNode} node - The node to get.
 * @returns {string|null} The property name if static. Otherwise, null.
 * @private
 */
function getStaticPropertyName(node) {
    let prop = null

    switch (node && node.type) {
        case "Property":
        case "MethodDefinition":
            prop = node.key
            break

        case "MemberExpression":
            prop = node.property
            break

        // no default
    }

    switch (prop && prop.type) {
        case "Literal":
            return String(prop.value)

        case "TemplateLiteral":
            if (prop.expressions.length === 0 && prop.quasis.length === 1) {
                return prop.quasis[0].value.cooked
            }
            break

        case "Identifier":
            if (!node.computed) {
                return prop.name
            }
            break

        // no default
    }

    return null
}

/**
 * Checks whether the given node is assignee or not.
 *
 * @param {ASTNode} node - The node to check.
 * @returns {boolean} `true` if the node is assignee.
 */
function isAssignee(node) {
    return (
        node.parent.type === "AssignmentExpression" && node.parent.left === node
    )
}

/**
 * Gets the top assignment expression node if the given node is an assignee.
 *
 * This is used to distinguish 2 assignees belong to the same assignment.
 * If the node is not an assignee, this returns null.
 *
 * @param {ASTNode} leafNode - The node to get.
 * @returns {ASTNode|null} The top assignment expression node, or null.
 */
function getTopAssignment(leafNode) {
    let node = leafNode

    // Skip MemberExpressions.
    while (
        node.parent.type === "MemberExpression" &&
        node.parent.object === node
    ) {
        node = node.parent
    }

    // Check assignments.
    if (!isAssignee(node)) {
        return null
    }

    // Find the top.
    while (node.parent.type === "AssignmentExpression") {
        node = node.parent
    }

    return node
}

/**
 * Gets top assignment nodes of the given node list.
 *
 * @param {ASTNode[]} nodes - The node list to get.
 * @returns {ASTNode[]} Gotten top assignment nodes.
 */
function createAssignmentList(nodes) {
    return nodes.map(getTopAssignment).filter(Boolean)
}

/**
 * Gets the reference of `module.exports` from the given scope.
 *
 * @param {escope.Scope} scope - The scope to get.
 * @returns {ASTNode[]} Gotten MemberExpression node list.
 */
function getModuleExportsNodes(scope) {
    const variable = scope.set.get("module")
    if (variable == null) {
        return []
    }
    return variable.references
        .map(reference => reference.identifier.parent)
        .filter(
            node =>
                node.type === "MemberExpression" &&
                getStaticPropertyName(node) === "exports"
        )
}

/**
 * Gets the reference of `exports` from the given scope.
 *
 * @param {escope.Scope} scope - The scope to get.
 * @returns {ASTNode[]} Gotten Identifier node list.
 */
function getExportsNodes(scope) {
    const variable = scope.set.get("exports")
    if (variable == null) {
        return []
    }
    return variable.references.map(reference => reference.identifier)
}

function getReplacementForProperty(property, sourceCode) {
    if (property.type !== "Property" || property.kind !== "init") {
        // We don't have a nice syntax for adding these directly on the exports object. Give up on fixing the whole thing:
        // property.kind === 'get':
        //   module.exports = { get foo() { ... } }
        // property.kind === 'set':
        //   module.exports = { set foo() { ... } }
        // property.type === 'SpreadElement':
        //   module.exports = { ...foo }
        return null
    }

    let fixedValue = sourceCode.getText(property.value)
    if (property.method) {
        fixedValue = `function${
            property.value.generator ? "*" : ""
        } ${fixedValue}`
        if (property.value.async) {
            fixedValue = `async ${fixedValue}`
        }
    }
    const lines = sourceCode
        .getCommentsBefore(property)
        .map(comment => sourceCode.getText(comment))
    if (property.key.type === "Literal" || property.computed) {
        // String or dynamic key:
        // module.exports = { [ ... ]: ... } or { "foo": ... }
        lines.push(
            `exports[${sourceCode.getText(property.key)}] = ${fixedValue};`
        )
    } else if (property.key.type === "Identifier") {
        // Regular identifier:
        // module.exports = { foo: ... }
        lines.push(`exports.${property.key.name} = ${fixedValue};`)
    } else {
        // Some other unknown property type. Conservatively give up on fixing the whole thing.
        return null
    }
    lines.push(
        ...sourceCode
            .getCommentsAfter(property)
            .map(comment => sourceCode.getText(comment))
    )
    return lines.join("\n")
}

// Check for a top level module.exports = { ... }
function isModuleExportsObjectAssignment(node) {
    return (
        node.parent.type === "AssignmentExpression" &&
        node.parent.parent.type === "ExpressionStatement" &&
        node.parent.parent.parent.type === "Program" &&
        node.parent.right.type === "ObjectExpression"
    )
}

// Check for module.exports.foo or module.exports.bar reference or assignment
function isModuleExportsReference(node) {
    return (
        node.parent.type === "MemberExpression" && node.parent.object === node
    )
}

function fixModuleExports(node, sourceCode, fixer) {
    if (isModuleExportsReference(node)) {
        return fixer.replaceText(node, "exports")
    }
    if (!isModuleExportsObjectAssignment(node)) {
        return null
    }
    const statements = []
    const properties = node.parent.right.properties
    for (const property of properties) {
        const statement = getReplacementForProperty(property, sourceCode)
        if (statement) {
            statements.push(statement)
        } else {
            // No replacement available, give up on the whole thing
            return null
        }
    }
    return fixer.replaceText(node.parent, statements.join("\n\n"))
}

module.exports = {
    meta: {
        docs: {
            description: "enforce either `module.exports` or `exports`",
            category: "Stylistic Issues",
            recommended: false,
            url: "https://github.com/weiran-zsd/eslint-plugin-node/blob/HEAD/docs/rules/exports-style.md",
        },
        type: "suggestion",
        fixable: "code",
        schema: [
            {
                //
                enum: ["module.exports", "exports"],
            },
            {
                type: "object",
                properties: { allowBatchAssign: { type: "boolean" } },
                additionalProperties: false,
            },
        ],
        messages: {
            unexpectedExports:
                "Unexpected access to 'exports'. Use 'module.exports' instead.",
            unexpectedModuleExports:
                "Unexpected access to 'module.exports'. Use 'exports' instead.",
            unexpectedAssignment:
                "Unexpected assignment to 'exports'. Don't modify 'exports' itself.",
        },
    },

    create(context) {
        const mode = context.options[0] || "module.exports"
        const batchAssignAllowed = Boolean(
            context.options[1] != null && context.options[1].allowBatchAssign
        )
        const sourceCode = context.getSourceCode()

        /**
         * Gets the location info of reports.
         *
         * exports = foo
         * ^^^^^^^^^
         *
         * module.exports = foo
         * ^^^^^^^^^^^^^^^^
         *
         * @param {ASTNode} node - The node of `exports`/`module.exports`.
         * @returns {Location} The location info of reports.
         */
        function getLocation(node) {
            const token = sourceCode.getTokenAfter(node)
            return {
                start: node.loc.start,
                end: token.loc.end,
            }
        }

        /**
         * Enforces `module.exports`.
         * This warns references of `exports`.
         *
         * @returns {void}
         */
        function enforceModuleExports() {
            const globalScope = context.getScope()
            const exportsNodes = getExportsNodes(globalScope)
            const assignList = batchAssignAllowed
                ? createAssignmentList(getModuleExportsNodes(globalScope))
                : []

            for (const node of exportsNodes) {
                // Skip if it's a batch assignment.
                if (
                    assignList.length > 0 &&
                    assignList.indexOf(getTopAssignment(node)) !== -1
                ) {
                    continue
                }

                // Report.
                context.report({
                    node,
                    loc: getLocation(node),
                    messageId: "unexpectedExports",
                })
            }
        }

        /**
         * Enforces `exports`.
         * This warns references of `module.exports`.
         *
         * @returns {void}
         */
        function enforceExports() {
            const globalScope = context.getScope()
            const exportsNodes = getExportsNodes(globalScope)
            const moduleExportsNodes = getModuleExportsNodes(globalScope)
            const assignList = batchAssignAllowed
                ? createAssignmentList(exportsNodes)
                : []
            const batchAssignList = []

            for (const node of moduleExportsNodes) {
                // Skip if it's a batch assignment.
                if (assignList.length > 0) {
                    const found = assignList.indexOf(getTopAssignment(node))
                    if (found !== -1) {
                        batchAssignList.push(assignList[found])
                        assignList.splice(found, 1)
                        continue
                    }
                }

                // Report.
                context.report({
                    node,
                    loc: getLocation(node),
                    messageId: "unexpectedModuleExports",
                    fix(fixer) {
                        return fixModuleExports(node, sourceCode, fixer)
                    },
                })
            }

            // Disallow direct assignment to `exports`.
            for (const node of exportsNodes) {
                // Skip if it's not assignee.
                if (!isAssignee(node)) {
                    continue
                }

                // Check if it's a batch assignment.
                if (batchAssignList.indexOf(getTopAssignment(node)) !== -1) {
                    continue
                }

                // Report.
                context.report({
                    node,
                    loc: getLocation(node),
                    messageId: "unexpectedAssignment",
                })
            }
        }

        return {
            "Program:exit"() {
                switch (mode) {
                    case "module.exports":
                        enforceModuleExports()
                        break
                    case "exports":
                        enforceExports()
                        break

                    // no default
                }
            },
        }
    },
}
