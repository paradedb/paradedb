/**
 * Rule: no-nesting
 * Avoid nesting your promises.
 */

'use strict'

const getDocsUrl = require('./lib/get-docs-url')
const hasPromiseCallback = require('./lib/has-promise-callback')
const isInsidePromise = require('./lib/is-inside-promise')

module.exports = {
  meta: {
    type: 'suggestion',
    docs: {
      url: getDocsUrl('no-nesting'),
    },
    schema: [],
  },
  create(context) {
    /**
     * Array of callback function scopes.
     * Scopes are in order closest to the current node.
     * @type {import('eslint').Scope.Scope[]}
     */
    const callbackScopes = []

    /**
     * @param {import('eslint').Scope.Scope} scope
     * @returns {Iterable<import('eslint').Scope.Reference>}
     */
    function* iterateDefinedReferences(scope) {
      for (const variable of scope.variables) {
        for (const reference of variable.references) {
          yield reference
        }
      }
    }

    return {
      ':function'(node) {
        if (isInsidePromise(node)) {
          callbackScopes.unshift(context.getScope())
        }
      },
      ':function:exit'(node) {
        if (isInsidePromise(node)) {
          callbackScopes.shift()
        }
      },
      CallExpression(node) {
        if (!hasPromiseCallback(node)) return
        if (!callbackScopes.length) {
          // The node is not in the callback function.
          return
        }

        // Checks if the argument callback uses variables defined in the closest callback function scope.
        //
        // e.g.
        // ```
        // doThing()
        //  .then(a => getB(a)
        //    .then(b => getC(a, b))
        //  )
        // ```
        //
        // In the above case, Since the variables it references are undef,
        // we cannot refactor the nesting like following:
        // ```
        // doThing()
        //  .then(a => getB(a))
        //  .then(b => getC(a, b))
        // ```
        //
        // However, `getD` can be refactored in the following:
        // ```
        // doThing()
        //   .then(a => getB(a)
        //     .then(b => getC(a, b)
        //       .then(c => getD(a, c))
        //     )
        //   )
        // ```
        // ↓
        // ```
        // doThing()
        //   .then(a => getB(a)
        //     .then(b => getC(a, b))
        //     .then(c => getD(a, c))
        //   )
        // ```
        // This is why we only check the closest callback function scope.
        //
        const closestCallbackScope = callbackScopes[0]
        for (const reference of iterateDefinedReferences(
          closestCallbackScope
        )) {
          if (
            node.arguments.some(
              (arg) =>
                arg.range[0] <= reference.identifier.range[0] &&
                reference.identifier.range[1] <= arg.range[1]
            )
          ) {
            // Argument callbacks refer to variables defined in the callback function.
            return
          }
        }

        context.report({
          node: node.callee.property,
          message: 'Avoid nesting promises.',
        })
      },
    }
  },
}
