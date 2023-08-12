/**
 * Library: Has Promise Callback
 * Makes sure that an Expression node is part of a promise
 * with callback functions (like then() or catch())
 */

'use strict'

/**
 * @typedef {import('estree').SimpleCallExpression} CallExpression
 * @typedef {import('estree').MemberExpression} MemberExpression
 * @typedef {import('estree').Identifier} Identifier
 *
 * @typedef {object} NameIsThenOrCatch
 * @property {'then' | 'catch'} name
 *
 * @typedef {object} PropertyIsThenOrCatch
 * @property {Identifier & NameIsThenOrCatch} property
 *
 * @typedef {object} CalleeIsPromiseCallback
 * @property {MemberExpression & PropertyIsThenOrCatch} callee
 *
 * @typedef {CallExpression & CalleeIsPromiseCallback} HasPromiseCallback
 */
/**
 * @param {import('estree').Node} node
 * @returns {node is HasPromiseCallback}
 */
function hasPromiseCallback(node) {
  // istanbul ignore if -- only being called within `CallExpression`
  if (node.type !== 'CallExpression') return
  if (node.callee.type !== 'MemberExpression') return
  const propertyName = node.callee.property.name
  return propertyName === 'then' || propertyName === 'catch'
}

module.exports = hasPromiseCallback
