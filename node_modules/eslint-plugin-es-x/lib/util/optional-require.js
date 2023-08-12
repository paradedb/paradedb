"use strict"

/**
 * Load a module optionally.
 * @param {Function} originalRequire The original `require` function.
 * @param {string} id The module specifier.
 */
function optionalRequire(originalRequire, id) {
    try {
        return originalRequire(id)
    } catch (error) {
        if (error && error.code === "MODULE_NOT_FOUND") {
            return undefined
        }
        throw error
    }
}

module.exports = { optionalRequire }
