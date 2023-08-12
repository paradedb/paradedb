'use strict'

const getDocsUrl = require('./lib/get-docs-url')

/**
 * @typedef {import('estree').Node} Node
 * @typedef {import('estree').SimpleCallExpression} CallExpression
 * @typedef {import('estree').FunctionExpression} FunctionExpression
 * @typedef {import('estree').ArrowFunctionExpression} ArrowFunctionExpression
 * @typedef {import('eslint').Rule.CodePath} CodePath
 * @typedef {import('eslint').Rule.CodePathSegment} CodePathSegment
 */

/**
 * @typedef { (FunctionExpression | ArrowFunctionExpression) & { parent: CallExpression }} InlineThenFunctionExpression
 */

/** @param {Node} node */
function isFunctionWithBlockStatement(node) {
  if (node.type === 'FunctionExpression') {
    return true
  }
  if (node.type === 'ArrowFunctionExpression') {
    return node.body.type === 'BlockStatement'
  }
  return false
}

/**
 * @param {string} memberName
 * @param {Node} node
 * @returns {node is CallExpression}
 */
function isMemberCall(memberName, node) {
  return (
    node.type === 'CallExpression' &&
    node.callee.type === 'MemberExpression' &&
    !node.callee.computed &&
    node.callee.property.type === 'Identifier' &&
    node.callee.property.name === memberName
  )
}

/** @param {Node} node */
function isFirstArgument(node) {
  return Boolean(
    node.parent && node.parent.arguments && node.parent.arguments[0] === node
  )
}

/**
 * @param {Node} node
 * @returns {node is InlineThenFunctionExpression}
 */
function isInlineThenFunctionExpression(node) {
  return (
    isFunctionWithBlockStatement(node) &&
    isMemberCall('then', node.parent) &&
    isFirstArgument(node)
  )
}

/**
 * Checks whether the given node is the last `then()` callback in a promise chain.
 * @param {InlineThenFunctionExpression} node
 */
function isLastCallback(node) {
  /** @type {Node} */
  let target = node.parent
  /** @type {Node | undefined} */
  let parent = target.parent
  while (parent) {
    if (parent.type === 'ExpressionStatement') {
      // e.g. { promise.then(() => value) }
      return true
    }
    if (parent.type === 'UnaryExpression') {
      // e.g. void promise.then(() => value)
      return parent.operator === 'void'
    }
    /** @type {Node | null} */
    let nextTarget = null
    if (parent.type === 'SequenceExpression') {
      if (peek(parent.expressions) !== target) {
        // e.g. (promise?.then(() => value), expr)
        return true
      }
      nextTarget = parent
    } else if (
      // e.g. promise?.then(() => value)
      parent.type === 'ChainExpression' ||
      // e.g. await promise.then(() => value)
      parent.type === 'AwaitExpression'
    ) {
      nextTarget = parent
    } else if (parent.type === 'MemberExpression') {
      if (
        parent.parent &&
        (isMemberCall('catch', parent.parent) ||
          isMemberCall('finally', parent.parent))
      ) {
        // e.g. promise.then(() => value).catch(e => {})
        nextTarget = parent.parent
      }
    }
    if (nextTarget) {
      target = nextTarget
      parent = target.parent
      continue
    }
    return false
  }

  // istanbul ignore next
  return false
}

/**
 * @template T
 * @param {T[]} arr
 * @returns {T}
 */
function peek(arr) {
  return arr[arr.length - 1]
}

module.exports = {
  meta: {
    type: 'problem',
    docs: {
      url: getDocsUrl('always-return'),
    },
    schema: [
      {
        type: 'object',
        properties: {
          ignoreLastCallback: {
            type: 'boolean',
          },
        },
        additionalProperties: false,
      },
    ],
  },
  create(context) {
    const options = context.options[0] || {}
    const ignoreLastCallback = !!options.ignoreLastCallback
    /**
     * @typedef {object} FuncInfo
     * @property {string[]} branchIDStack This is a stack representing the currently
     *   executing branches ("codePathSegment"s) within the given function
     * @property {Record<string, BranchInfo | undefined>} branchInfoMap This is an object representing information
     *   about all branches within the given function
     *
     * @typedef {object} BranchInfo
     * @property {boolean} good This is a boolean representing whether
     *   the given branch explicitly `return`s or `throw`s. It starts as `false`
     *   for every branch and is updated to `true` if a `return` or `throw`
     *   statement is found
     * @property {Node} node This is a estree Node object
     *   for the given branch
     */

    /**
     * funcInfoStack is a stack representing the stack of currently executing
     *   functions
     * example:
     *   funcInfoStack = [ { branchIDStack: [ 's1_1' ],
     *       branchInfoMap:
     *        { s1_1:
     *           { good: false,
     *             loc: <loc> } } },
     *     { branchIDStack: ['s2_1', 's2_4'],
     *       branchInfoMap:
     *        { s2_1:
     *           { good: false,
     *             loc: <loc> },
     *          s2_2:
     *           { good: true,
     *             loc: <loc> },
     *          s2_4:
     *           { good: false,
     *             loc: <loc> } } } ]
     * @type {FuncInfo[]}
     */
    const funcInfoStack = []

    function markCurrentBranchAsGood() {
      const funcInfo = peek(funcInfoStack)
      const currentBranchID = peek(funcInfo.branchIDStack)
      if (funcInfo.branchInfoMap[currentBranchID]) {
        funcInfo.branchInfoMap[currentBranchID].good = true
      }
      // else unreachable code
    }

    return {
      'ReturnStatement:exit': markCurrentBranchAsGood,
      'ThrowStatement:exit': markCurrentBranchAsGood,

      /**
       * @param {CodePathSegment} segment
       * @param {Node} node
       */
      onCodePathSegmentStart(segment, node) {
        const funcInfo = peek(funcInfoStack)
        funcInfo.branchIDStack.push(segment.id)
        funcInfo.branchInfoMap[segment.id] = { good: false, node }
      },

      onCodePathSegmentEnd() {
        const funcInfo = peek(funcInfoStack)
        funcInfo.branchIDStack.pop()
      },

      onCodePathStart() {
        funcInfoStack.push({
          branchIDStack: [],
          branchInfoMap: {},
        })
      },

      /**
       * @param {CodePath} path
       * @param {Node} node
       */
      onCodePathEnd(path, node) {
        const funcInfo = funcInfoStack.pop()

        if (!isInlineThenFunctionExpression(node)) {
          return
        }

        if (ignoreLastCallback && isLastCallback(node)) {
          return
        }

        path.finalSegments.forEach((segment) => {
          const id = segment.id
          const branch = funcInfo.branchInfoMap[id]
          if (!branch.good) {
            context.report({
              message: 'Each then() should return a value or throw',
              node: branch.node,
            })
          }
        })
      },
    }
  },
}
