/**
 * @author Toru Nagashima
 * See LICENSE file in root directory for full license.
 */
"use strict"

const path = require("path")
const { pathToFileURL, fileURLToPath } = require("url")
const resolve = require("resolve")
const {
    defaultResolve: importResolve,
} = require("../converted-esm/import-meta-resolve")

/**
 * Resolve the given id to file paths.
 * @param {boolean} isModule The flag which indicates this id is a module.
 * @param {string} id The id to resolve.
 * @param {object} options The options of node-resolve module.
 * It requires `options.basedir`.
 * @param {'import' | 'require'} moduleType - whether the target was require-ed or imported
 * @returns {string|null} The resolved path.
 */
function getFilePath(isModule, id, options, moduleType) {
    if (moduleType === "import") {
        const paths =
            options.paths && options.paths.length > 0
                ? options.paths.map(p => path.resolve(process.cwd(), p))
                : [options.basedir]
        for (const aPath of paths) {
            try {
                const { url } = importResolve(id, {
                    parentURL: pathToFileURL(path.join(aPath, "dummy-file.mjs"))
                        .href,
                    conditions: ["node", "import", "require"],
                })

                if (url) {
                    return fileURLToPath(url)
                }
            } catch (e) {
                continue
            }
        }

        if (isModule) {
            return null
        }
        return path.resolve(
            (options.paths && options.paths[0]) || options.basedir,
            id
        )
    } else {
        try {
            return resolve.sync(id, options)
        } catch (_err) {
            try {
                const { url } = importResolve(id, {
                    parentURL: pathToFileURL(
                        path.join(options.basedir, "dummy-file.js")
                    ).href,
                    conditions: ["node", "require"],
                })

                return fileURLToPath(url)
            } catch (err) {
                if (isModule) {
                    return null
                }
                return path.resolve(options.basedir, id)
            }
        }
    }
}

/**
 * Gets the module name of a given path.
 *
 * e.g. `eslint/lib/ast-utils` -> `eslint`
 *
 * @param {string} nameOrPath - A path to get.
 * @returns {string} The module name of the path.
 */
function getModuleName(nameOrPath) {
    let end = nameOrPath.indexOf("/")
    if (end !== -1 && nameOrPath[0] === "@") {
        end = nameOrPath.indexOf("/", 1 + end)
    }

    return end === -1 ? nameOrPath : nameOrPath.slice(0, end)
}

/**
 * Information of an import target.
 */
module.exports = class ImportTarget {
    /**
     * Initialize this instance.
     * @param {ASTNode} node - The node of a `require()` or a module declaraiton.
     * @param {string} name - The name of an import target.
     * @param {object} options - The options of `node-resolve` module.
     * @param {'import' | 'require'} moduleType - whether the target was require-ed or imported
     */
    constructor(node, name, options, moduleType) {
        const isModule = !/^(?:[./\\]|\w+:)/u.test(name)

        /**
         * The node of a `require()` or a module declaraiton.
         * @type {ASTNode}
         */
        this.node = node

        /**
         * The name of this import target.
         * @type {string}
         */
        this.name = name

        /**
         * The full path of this import target.
         * If the target is a module and it does not exist then this is `null`.
         * @type {string|null}
         */
        this.filePath = getFilePath(isModule, name, options, moduleType)

        /**
         * The module name of this import target.
         * If the target is a relative path then this is `null`.
         * @type {string|null}
         */
        this.moduleName = isModule ? getModuleName(name) : null
    }
}
