"use strict"

module.exports = {
    meta: {
        docs: {
            description:
                "disallow identifiers from shadowing catch parameter names.",
            category: "legacy",
            recommended: false,
            url: "http://eslint-community.github.io/eslint-plugin-es-x/rules/no-shadow-catch-param.html",
        },
        fixable: null,
        messages: {
            forbidden: "Shadowing of catch parameter '{{name}}'.",
        },
        schema: [],
        type: "problem",
    },
    create(context) {
        return {
            "CatchClause > Identifier.param:exit"(node) {
                const scope = context.getScope()
                const shadowingVar = scope.variableScope.set.get(node.name)
                if (!shadowingVar) {
                    return
                }
                for (const def of shadowingVar.defs) {
                    if (def.type !== "Variable") {
                        continue
                    }
                    const varDecl = def.node
                    if (varDecl.parent.kind !== "var") {
                        continue
                    }
                    const varId = varDecl.id
                    const catchClause = node.parent
                    const bodyRange = catchClause.body.range
                    if (
                        bodyRange[0] <= varId.range[0] &&
                        varId.range[1] <= bodyRange[1]
                    ) {
                        context.report({
                            node: varDecl,
                            messageId: "forbidden",
                            data: {
                                name: node.name,
                            },
                        })
                    }
                }
            },
        }
    },
}
