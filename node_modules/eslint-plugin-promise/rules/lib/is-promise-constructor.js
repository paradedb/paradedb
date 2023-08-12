/**
 * Library: isPromiseConstructor
 * Makes sure that an Expression node is new Promise().
 */
'use strict'

/**
 * @typedef {import('estree').Node} Node
 * @typedef {import('estree').Expression} Expression
 * @typedef {import('estree').NewExpression} NewExpression
 * @typedef {import('estree').FunctionExpression} FunctionExpression
 * @typedef {import('estree').ArrowFunctionExpression} ArrowFunctionExpression
 *
 * @typedef {NewExpression & { callee: { type: 'Identifier', name: 'Promise' } }} NewPromise
 * @typedef {NewPromise & { arguments: [FunctionExpression | ArrowFunctionExpression] }} NewPromiseWithInlineExecutor
 *
 */
/**
 * Checks whether the given node is new Promise().
 * @param {Node} node
 * @returns {node is NewPromise}
 */
function isPromiseConstructor(node) {
  return (
    node.type === 'NewExpression' &&
    node.callee.type === 'Identifier' &&
    node.callee.name === 'Promise'
  )
}

/**
 * Checks whether the given node is new Promise(() => {}).
 * @param {Node} node
 * @returns {node is NewPromiseWithInlineExecutor}
 */
function isPromiseConstructorWithInlineExecutor(node) {
  return (
    isPromiseConstructor(node) &&
    node.arguments.length === 1 &&
    (node.arguments[0].type === 'FunctionExpression' ||
      node.arguments[0].type === 'ArrowFunctionExpression')
  )
}

module.exports = {
  isPromiseConstructor,
  isPromiseConstructorWithInlineExecutor,
}
