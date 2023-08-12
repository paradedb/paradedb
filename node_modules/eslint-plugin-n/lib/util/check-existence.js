/**
 * @author Toru Nagashima
 * See LICENSE file in root directory for full license.
 */
"use strict"

const path = require("path")
const exists = require("./exists")
const getAllowModules = require("./get-allow-modules")
const isTypescript = require("./is-typescript")
const mapTypescriptExtension = require("../util/map-typescript-extension")

/**
 * Checks whether or not each requirement target exists.
 *
 * It looks up the target according to the logic of Node.js.
 * See Also: https://nodejs.org/api/modules.html
 *
 * @param {RuleContext} context - A context to report.
 * @param {ImportTarget[]} targets - A list of target information to check.
 * @returns {void}
 */
exports.checkExistence = function checkExistence(context, targets) {
    const allowed = new Set(getAllowModules(context))

    for (const target of targets) {
        const missingModule =
            target.moduleName != null &&
            !allowed.has(target.moduleName) &&
            target.filePath == null

        let missingFile = target.moduleName == null && !exists(target.filePath)
        if (missingFile && isTypescript(context)) {
            const parsed = path.parse(target.filePath)
            const reversedExt = mapTypescriptExtension(
                context,
                target.filePath,
                parsed.ext,
                true
            )
            const reversedPath =
                path.resolve(parsed.dir, parsed.name) + reversedExt
            missingFile = target.moduleName == null && !exists(reversedPath)
        }

        if (missingModule || missingFile) {
            context.report({
                node: target.node,
                loc: target.node.loc,
                messageId: "notFound",
                data: target,
            })
        }
    }
}

exports.messages = {
    notFound: '"{{name}}" is not found.',
}
