/**
 * @author Toru Nagashima
 * See LICENSE file in root directory for full license.
 */
"use strict"

const path = require("path")
const getAllowModules = require("./get-allow-modules")
const getConvertPath = require("./get-convert-path")
const getNpmignore = require("./get-npmignore")
const getPackageJson = require("./get-package-json")

/**
 * Checks whether or not each requirement target is published via package.json.
 *
 * It reads package.json and checks the target exists in `dependencies`.
 *
 * @param {RuleContext} context - A context to report.
 * @param {string} filePath - The current file path.
 * @param {ImportTarget[]} targets - A list of target information to check.
 * @returns {void}
 */
exports.checkPublish = function checkPublish(context, filePath, targets) {
    const packageInfo = getPackageJson(filePath)
    if (!packageInfo) {
        return
    }

    // Private packages are never published so we don't need to check the imported dependencies either.
    // More information: https://docs.npmjs.com/cli/v8/configuring-npm/package-json#private
    if (packageInfo.private === true) {
        return
    }

    const allowed = new Set(getAllowModules(context))
    const convertPath = getConvertPath(context)
    const basedir = path.dirname(packageInfo.filePath)

    const toRelative = fullPath => {
        const retv = path.relative(basedir, fullPath).replace(/\\/gu, "/")
        return convertPath(retv)
    }
    const npmignore = getNpmignore(filePath)
    const devDependencies = new Set(
        Object.keys(packageInfo.devDependencies || {})
    )
    const dependencies = new Set(
        [].concat(
            Object.keys(packageInfo.dependencies || {}),
            Object.keys(packageInfo.peerDependencies || {}),
            Object.keys(packageInfo.optionalDependencies || {})
        )
    )

    if (!npmignore.match(toRelative(filePath))) {
        // This file is published, so this cannot import private files.
        for (const target of targets) {
            const isPrivateFile = () => {
                if (target.moduleName != null) {
                    return false
                }
                const relativeTargetPath = toRelative(target.filePath)
                return (
                    relativeTargetPath !== "" &&
                    npmignore.match(relativeTargetPath)
                )
            }
            const isDevPackage = () =>
                target.moduleName != null &&
                devDependencies.has(target.moduleName) &&
                !dependencies.has(target.moduleName) &&
                !allowed.has(target.moduleName)
            if (isPrivateFile() || isDevPackage()) {
                context.report({
                    node: target.node,
                    loc: target.node.loc,
                    messageId: "notPublished",
                    data: { name: target.moduleName || target.name },
                })
            }
        }
    }
}

exports.messages = {
    notPublished: '"{{name}}" is not published.',
}
