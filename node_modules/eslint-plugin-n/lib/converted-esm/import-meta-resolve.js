var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// node_modules/import-meta-resolve/lib/resolve.js
var resolve_exports = {};
__export(resolve_exports, {
  defaultResolve: () => defaultResolve,
  moduleResolve: () => moduleResolve
});
module.exports = __toCommonJS(resolve_exports);
var import_node_assert2 = __toESM(require("node:assert"), 1);
var import_node_fs2 = require("node:fs");
var import_node_process2 = __toESM(require("node:process"), 1);
var import_node_url3 = require("node:url");
var import_node_path3 = __toESM(require("node:path"), 1);
var import_node_module = require("node:module");

// node_modules/import-meta-resolve/lib/get-format.js
var import_node_path2 = __toESM(require("node:path"), 1);
var import_node_url2 = require("node:url");

// node_modules/import-meta-resolve/lib/package-config.js
var import_node_url = require("node:url");

// node_modules/import-meta-resolve/lib/errors.js
var import_node_v8 = __toESM(require("node:v8"), 1);
var import_node_process = __toESM(require("node:process"), 1);
var import_node_assert = __toESM(require("node:assert"), 1);
var import_node_util = require("node:util");
var isWindows = import_node_process.default.platform === "win32";
var own = {}.hasOwnProperty;
var codes = {};
function formatList(array, type = "and") {
  return array.length < 3 ? array.join(` ${type} `) : `${array.slice(0, -1).join(", ")}, ${type} ${array[array.length - 1]}`;
}
var messages = /* @__PURE__ */ new Map();
var nodeInternalPrefix = "__node_internal_";
var userStackTraceLimit;
codes.ERR_INVALID_MODULE_SPECIFIER = createError(
  "ERR_INVALID_MODULE_SPECIFIER",
  /**
   * @param {string} request
   * @param {string} reason
   * @param {string} [base]
   */
  (request, reason, base = void 0) => {
    return `Invalid module "${request}" ${reason}${base ? ` imported from ${base}` : ""}`;
  },
  TypeError
);
codes.ERR_INVALID_PACKAGE_CONFIG = createError(
  "ERR_INVALID_PACKAGE_CONFIG",
  /**
   * @param {string} path
   * @param {string} [base]
   * @param {string} [message]
   */
  (path4, base, message) => {
    return `Invalid package config ${path4}${base ? ` while importing ${base}` : ""}${message ? `. ${message}` : ""}`;
  },
  Error
);
codes.ERR_INVALID_PACKAGE_TARGET = createError(
  "ERR_INVALID_PACKAGE_TARGET",
  /**
   * @param {string} pkgPath
   * @param {string} key
   * @param {unknown} target
   * @param {boolean} [isImport=false]
   * @param {string} [base]
   */
  (pkgPath, key, target, isImport = false, base = void 0) => {
    const relError = typeof target === "string" && !isImport && target.length > 0 && !target.startsWith("./");
    if (key === ".") {
      (0, import_node_assert.default)(isImport === false);
      return `Invalid "exports" main target ${JSON.stringify(target)} defined in the package config ${pkgPath}package.json${base ? ` imported from ${base}` : ""}${relError ? '; targets must start with "./"' : ""}`;
    }
    return `Invalid "${isImport ? "imports" : "exports"}" target ${JSON.stringify(
      target
    )} defined for '${key}' in the package config ${pkgPath}package.json${base ? ` imported from ${base}` : ""}${relError ? '; targets must start with "./"' : ""}`;
  },
  Error
);
codes.ERR_MODULE_NOT_FOUND = createError(
  "ERR_MODULE_NOT_FOUND",
  /**
   * @param {string} path
   * @param {string} base
   * @param {string} [type]
   */
  (path4, base, type = "package") => {
    return `Cannot find ${type} '${path4}' imported from ${base}`;
  },
  Error
);
codes.ERR_NETWORK_IMPORT_DISALLOWED = createError(
  "ERR_NETWORK_IMPORT_DISALLOWED",
  "import of '%s' by %s is not supported: %s",
  Error
);
codes.ERR_PACKAGE_IMPORT_NOT_DEFINED = createError(
  "ERR_PACKAGE_IMPORT_NOT_DEFINED",
  /**
   * @param {string} specifier
   * @param {string} packagePath
   * @param {string} base
   */
  (specifier, packagePath, base) => {
    return `Package import specifier "${specifier}" is not defined${packagePath ? ` in package ${packagePath}package.json` : ""} imported from ${base}`;
  },
  TypeError
);
codes.ERR_PACKAGE_PATH_NOT_EXPORTED = createError(
  "ERR_PACKAGE_PATH_NOT_EXPORTED",
  /**
   * @param {string} pkgPath
   * @param {string} subpath
   * @param {string} [base]
   */
  (pkgPath, subpath, base = void 0) => {
    if (subpath === ".")
      return `No "exports" main defined in ${pkgPath}package.json${base ? ` imported from ${base}` : ""}`;
    return `Package subpath '${subpath}' is not defined by "exports" in ${pkgPath}package.json${base ? ` imported from ${base}` : ""}`;
  },
  Error
);
codes.ERR_UNSUPPORTED_DIR_IMPORT = createError(
  "ERR_UNSUPPORTED_DIR_IMPORT",
  "Directory import '%s' is not supported resolving ES modules imported from %s",
  Error
);
codes.ERR_UNKNOWN_FILE_EXTENSION = createError(
  "ERR_UNKNOWN_FILE_EXTENSION",
  /**
   * @param {string} ext
   * @param {string} path
   */
  (ext, path4) => {
    return `Unknown file extension "${ext}" for ${path4}`;
  },
  TypeError
);
codes.ERR_INVALID_ARG_VALUE = createError(
  "ERR_INVALID_ARG_VALUE",
  /**
   * @param {string} name
   * @param {unknown} value
   * @param {string} [reason='is invalid']
   */
  (name, value, reason = "is invalid") => {
    let inspected = (0, import_node_util.inspect)(value);
    if (inspected.length > 128) {
      inspected = `${inspected.slice(0, 128)}...`;
    }
    const type = name.includes(".") ? "property" : "argument";
    return `The ${type} '${name}' ${reason}. Received ${inspected}`;
  },
  TypeError
  // Note: extra classes have been shaken out.
  // , RangeError
);
codes.ERR_UNSUPPORTED_ESM_URL_SCHEME = createError(
  "ERR_UNSUPPORTED_ESM_URL_SCHEME",
  /**
   * @param {URL} url
   * @param {Array<string>} supported
   */
  (url, supported) => {
    let message = `Only URLs with a scheme in: ${formatList(
      supported
    )} are supported by the default ESM loader`;
    if (isWindows && url.protocol.length === 2) {
      message += ". On Windows, absolute paths must be valid file:// URLs";
    }
    message += `. Received protocol '${url.protocol}'`;
    return message;
  },
  Error
);
function createError(sym, value, def) {
  messages.set(sym, value);
  return makeNodeErrorWithCode(def, sym);
}
function makeNodeErrorWithCode(Base, key) {
  return NodeError;
  function NodeError(...args) {
    const limit = Error.stackTraceLimit;
    if (isErrorStackTraceLimitWritable())
      Error.stackTraceLimit = 0;
    const error = new Base();
    if (isErrorStackTraceLimitWritable())
      Error.stackTraceLimit = limit;
    const message = getMessage(key, args, error);
    Object.defineProperties(error, {
      // Note: no need to implement `kIsNodeError` symbol, would be hard,
      // probably.
      message: {
        value: message,
        enumerable: false,
        writable: true,
        configurable: true
      },
      toString: {
        /** @this {Error} */
        value() {
          return `${this.name} [${key}]: ${this.message}`;
        },
        enumerable: false,
        writable: true,
        configurable: true
      }
    });
    captureLargerStackTrace(error);
    error.code = key;
    return error;
  }
}
function isErrorStackTraceLimitWritable() {
  try {
    if (import_node_v8.default.startupSnapshot.isBuildingSnapshot()) {
      return false;
    }
  } catch {
  }
  const desc = Object.getOwnPropertyDescriptor(Error, "stackTraceLimit");
  if (desc === void 0) {
    return Object.isExtensible(Error);
  }
  return own.call(desc, "writable") && desc.writable !== void 0 ? desc.writable : desc.set !== void 0;
}
function hideStackFrames(fn) {
  const hidden = nodeInternalPrefix + fn.name;
  Object.defineProperty(fn, "name", { value: hidden });
  return fn;
}
var captureLargerStackTrace = hideStackFrames(
  /**
   * @param {Error} error
   * @returns {Error}
   */
  // @ts-expect-error: fine
  function(error) {
    const stackTraceLimitIsWritable = isErrorStackTraceLimitWritable();
    if (stackTraceLimitIsWritable) {
      userStackTraceLimit = Error.stackTraceLimit;
      Error.stackTraceLimit = Number.POSITIVE_INFINITY;
    }
    Error.captureStackTrace(error);
    if (stackTraceLimitIsWritable)
      Error.stackTraceLimit = userStackTraceLimit;
    return error;
  }
);
function getMessage(key, args, self) {
  const message = messages.get(key);
  (0, import_node_assert.default)(typeof message !== "undefined", "expected `message` to be found");
  if (typeof message === "function") {
    (0, import_node_assert.default)(
      message.length <= args.length,
      // Default options do not count.
      `Code: ${key}; The provided arguments length (${args.length}) does not match the required ones (${message.length}).`
    );
    return Reflect.apply(message, self, args);
  }
  const regex = /%[dfijoOs]/g;
  let expectedLength = 0;
  while (regex.exec(message) !== null)
    expectedLength++;
  (0, import_node_assert.default)(
    expectedLength === args.length,
    `Code: ${key}; The provided arguments length (${args.length}) does not match the required ones (${expectedLength}).`
  );
  if (args.length === 0)
    return message;
  args.unshift(message);
  return Reflect.apply(import_node_util.format, null, args);
}

// node_modules/import-meta-resolve/lib/package-json-reader.js
var import_node_fs = __toESM(require("node:fs"), 1);
var import_node_path = __toESM(require("node:path"), 1);
var reader = { read };
var package_json_reader_default = reader;
function read(jsonPath) {
  try {
    const string = import_node_fs.default.readFileSync(
      import_node_path.default.toNamespacedPath(import_node_path.default.join(import_node_path.default.dirname(jsonPath), "package.json")),
      "utf8"
    );
    return { string };
  } catch (error) {
    const exception = (
      /** @type {ErrnoException} */
      error
    );
    if (exception.code === "ENOENT") {
      return { string: void 0 };
    }
    throw exception;
  }
}

// node_modules/import-meta-resolve/lib/package-config.js
var { ERR_INVALID_PACKAGE_CONFIG } = codes;
var packageJsonCache = /* @__PURE__ */ new Map();
function getPackageConfig(path4, specifier, base) {
  const existing = packageJsonCache.get(path4);
  if (existing !== void 0) {
    return existing;
  }
  const source = package_json_reader_default.read(path4).string;
  if (source === void 0) {
    const packageConfig2 = {
      pjsonPath: path4,
      exists: false,
      main: void 0,
      name: void 0,
      type: "none",
      exports: void 0,
      imports: void 0
    };
    packageJsonCache.set(path4, packageConfig2);
    return packageConfig2;
  }
  let packageJson;
  try {
    packageJson = JSON.parse(source);
  } catch (error) {
    const exception = (
      /** @type {ErrnoException} */
      error
    );
    throw new ERR_INVALID_PACKAGE_CONFIG(
      path4,
      (base ? `"${specifier}" from ` : "") + (0, import_node_url.fileURLToPath)(base || specifier),
      exception.message
    );
  }
  const { exports, imports, main, name, type } = packageJson;
  const packageConfig = {
    pjsonPath: path4,
    exists: true,
    main: typeof main === "string" ? main : void 0,
    name: typeof name === "string" ? name : void 0,
    type: type === "module" || type === "commonjs" ? type : "none",
    // @ts-expect-error Assume `Record<string, unknown>`.
    exports,
    // @ts-expect-error Assume `Record<string, unknown>`.
    imports: imports && typeof imports === "object" ? imports : void 0
  };
  packageJsonCache.set(path4, packageConfig);
  return packageConfig;
}
function getPackageScopeConfig(resolved) {
  let packageJsonUrl = new import_node_url.URL("package.json", resolved);
  while (true) {
    const packageJsonPath2 = packageJsonUrl.pathname;
    if (packageJsonPath2.endsWith("node_modules/package.json"))
      break;
    const packageConfig2 = getPackageConfig(
      (0, import_node_url.fileURLToPath)(packageJsonUrl),
      resolved
    );
    if (packageConfig2.exists)
      return packageConfig2;
    const lastPackageJsonUrl = packageJsonUrl;
    packageJsonUrl = new import_node_url.URL("../package.json", packageJsonUrl);
    if (packageJsonUrl.pathname === lastPackageJsonUrl.pathname)
      break;
  }
  const packageJsonPath = (0, import_node_url.fileURLToPath)(packageJsonUrl);
  const packageConfig = {
    pjsonPath: packageJsonPath,
    exists: false,
    main: void 0,
    name: void 0,
    type: "none",
    exports: void 0,
    imports: void 0
  };
  packageJsonCache.set(packageJsonPath, packageConfig);
  return packageConfig;
}

// node_modules/import-meta-resolve/lib/resolve-get-package-type.js
function getPackageType(url) {
  const packageConfig = getPackageScopeConfig(url);
  return packageConfig.type;
}

// node_modules/import-meta-resolve/lib/get-format.js
var { ERR_UNKNOWN_FILE_EXTENSION } = codes;
var hasOwnProperty = {}.hasOwnProperty;
var extensionFormatMap = {
  // @ts-expect-error: hush.
  __proto__: null,
  ".cjs": "commonjs",
  ".js": "module",
  ".json": "json",
  ".mjs": "module"
};
function mimeToFormat(mime) {
  if (mime && /\s*(text|application)\/javascript\s*(;\s*charset=utf-?8\s*)?/i.test(mime))
    return "module";
  if (mime === "application/json")
    return "json";
  return null;
}
var protocolHandlers = {
  // @ts-expect-error: hush.
  __proto__: null,
  "data:": getDataProtocolModuleFormat,
  "file:": getFileProtocolModuleFormat,
  "http:": getHttpProtocolModuleFormat,
  "https:": getHttpProtocolModuleFormat,
  "node:"() {
    return "builtin";
  }
};
function getDataProtocolModuleFormat(parsed) {
  const { 1: mime } = /^([^/]+\/[^;,]+)[^,]*?(;base64)?,/.exec(
    parsed.pathname
  ) || [null, null, null];
  return mimeToFormat(mime);
}
function getFileProtocolModuleFormat(url, _context, ignoreErrors) {
  const filepath = (0, import_node_url2.fileURLToPath)(url);
  const ext = import_node_path2.default.extname(filepath);
  if (ext === ".js") {
    return getPackageType(url) === "module" ? "module" : "commonjs";
  }
  const format2 = extensionFormatMap[ext];
  if (format2)
    return format2;
  if (ignoreErrors) {
    return void 0;
  }
  throw new ERR_UNKNOWN_FILE_EXTENSION(ext, filepath);
}
function getHttpProtocolModuleFormat() {
}
function defaultGetFormatWithoutErrors(url, context) {
  if (!hasOwnProperty.call(protocolHandlers, url.protocol)) {
    return null;
  }
  return protocolHandlers[url.protocol](url, context, true) || null;
}

// node_modules/import-meta-resolve/lib/utils.js
var { ERR_INVALID_ARG_VALUE } = codes;
var DEFAULT_CONDITIONS = Object.freeze(["node", "import"]);
var DEFAULT_CONDITIONS_SET = new Set(DEFAULT_CONDITIONS);
function getDefaultConditions() {
  return DEFAULT_CONDITIONS;
}
function getDefaultConditionsSet() {
  return DEFAULT_CONDITIONS_SET;
}
function getConditionsSet(conditions) {
  if (conditions !== void 0 && conditions !== getDefaultConditions()) {
    if (!Array.isArray(conditions)) {
      throw new ERR_INVALID_ARG_VALUE(
        "conditions",
        conditions,
        "expected an array"
      );
    }
    return new Set(conditions);
  }
  return getDefaultConditionsSet();
}

// node_modules/import-meta-resolve/lib/resolve.js
var RegExpPrototypeSymbolReplace = RegExp.prototype[Symbol.replace];
var experimentalNetworkImports = false;
var {
  ERR_NETWORK_IMPORT_DISALLOWED,
  ERR_INVALID_MODULE_SPECIFIER,
  ERR_INVALID_PACKAGE_CONFIG: ERR_INVALID_PACKAGE_CONFIG2,
  ERR_INVALID_PACKAGE_TARGET,
  ERR_MODULE_NOT_FOUND,
  ERR_PACKAGE_IMPORT_NOT_DEFINED,
  ERR_PACKAGE_PATH_NOT_EXPORTED,
  ERR_UNSUPPORTED_DIR_IMPORT,
  ERR_UNSUPPORTED_ESM_URL_SCHEME
} = codes;
var own2 = {}.hasOwnProperty;
var invalidSegmentRegEx = /(^|\\|\/)((\.|%2e)(\.|%2e)?|(n|%6e|%4e)(o|%6f|%4f)(d|%64|%44)(e|%65|%45)(_|%5f)(m|%6d|%4d)(o|%6f|%4f)(d|%64|%44)(u|%75|%55)(l|%6c|%4c)(e|%65|%45)(s|%73|%53))?(\\|\/|$)/i;
var deprecatedInvalidSegmentRegEx = /(^|\\|\/)((\.|%2e)(\.|%2e)?|(n|%6e|%4e)(o|%6f|%4f)(d|%64|%44)(e|%65|%45)(_|%5f)(m|%6d|%4d)(o|%6f|%4f)(d|%64|%44)(u|%75|%55)(l|%6c|%4c)(e|%65|%45)(s|%73|%53))(\\|\/|$)/i;
var invalidPackageNameRegEx = /^\.|%|\\/;
var patternRegEx = /\*/g;
var encodedSepRegEx = /%2f|%5c/i;
var emittedPackageWarnings = /* @__PURE__ */ new Set();
var doubleSlashRegEx = /[/\\]{2}/;
function emitInvalidSegmentDeprecation(target, request, match, packageJsonUrl, internal, base, isTarget) {
  const pjsonPath = (0, import_node_url3.fileURLToPath)(packageJsonUrl);
  const double = doubleSlashRegEx.exec(isTarget ? target : request) !== null;
  import_node_process2.default.emitWarning(
    `Use of deprecated ${double ? "double slash" : "leading or trailing slash matching"} resolving "${target}" for module request "${request}" ${request === match ? "" : `matched to "${match}" `}in the "${internal ? "imports" : "exports"}" field module resolution of the package at ${pjsonPath}${base ? ` imported from ${(0, import_node_url3.fileURLToPath)(base)}` : ""}.`,
    "DeprecationWarning",
    "DEP0166"
  );
}
function emitLegacyIndexDeprecation(url, packageJsonUrl, base, main) {
  const format2 = defaultGetFormatWithoutErrors(url, { parentURL: base.href });
  if (format2 !== "module")
    return;
  const path4 = (0, import_node_url3.fileURLToPath)(url.href);
  const pkgPath = (0, import_node_url3.fileURLToPath)(new import_node_url3.URL(".", packageJsonUrl));
  const basePath = (0, import_node_url3.fileURLToPath)(base);
  if (main)
    import_node_process2.default.emitWarning(
      `Package ${pkgPath} has a "main" field set to ${JSON.stringify(main)}, excluding the full filename and extension to the resolved file at "${path4.slice(
        pkgPath.length
      )}", imported from ${basePath}.
 Automatic extension resolution of the "main" field isdeprecated for ES modules.`,
      "DeprecationWarning",
      "DEP0151"
    );
  else
    import_node_process2.default.emitWarning(
      `No "main" or "exports" field defined in the package.json for ${pkgPath} resolving the main entry point "${path4.slice(
        pkgPath.length
      )}", imported from ${basePath}.
Default "index" lookups for the main are deprecated for ES modules.`,
      "DeprecationWarning",
      "DEP0151"
    );
}
function tryStatSync(path4) {
  try {
    return (0, import_node_fs2.statSync)(path4);
  } catch {
    return new import_node_fs2.Stats();
  }
}
function fileExists(url) {
  const stats = (0, import_node_fs2.statSync)(url, { throwIfNoEntry: false });
  const isFile = stats ? stats.isFile() : void 0;
  return isFile === null || isFile === void 0 ? false : isFile;
}
function legacyMainResolve(packageJsonUrl, packageConfig, base) {
  let guess;
  if (packageConfig.main !== void 0) {
    guess = new import_node_url3.URL(packageConfig.main, packageJsonUrl);
    if (fileExists(guess))
      return guess;
    const tries2 = [
      `./${packageConfig.main}.js`,
      `./${packageConfig.main}.json`,
      `./${packageConfig.main}.node`,
      `./${packageConfig.main}/index.js`,
      `./${packageConfig.main}/index.json`,
      `./${packageConfig.main}/index.node`
    ];
    let i2 = -1;
    while (++i2 < tries2.length) {
      guess = new import_node_url3.URL(tries2[i2], packageJsonUrl);
      if (fileExists(guess))
        break;
      guess = void 0;
    }
    if (guess) {
      emitLegacyIndexDeprecation(
        guess,
        packageJsonUrl,
        base,
        packageConfig.main
      );
      return guess;
    }
  }
  const tries = ["./index.js", "./index.json", "./index.node"];
  let i = -1;
  while (++i < tries.length) {
    guess = new import_node_url3.URL(tries[i], packageJsonUrl);
    if (fileExists(guess))
      break;
    guess = void 0;
  }
  if (guess) {
    emitLegacyIndexDeprecation(guess, packageJsonUrl, base, packageConfig.main);
    return guess;
  }
  throw new ERR_MODULE_NOT_FOUND(
    (0, import_node_url3.fileURLToPath)(new import_node_url3.URL(".", packageJsonUrl)),
    (0, import_node_url3.fileURLToPath)(base)
  );
}
function finalizeResolution(resolved, base, preserveSymlinks) {
  if (encodedSepRegEx.exec(resolved.pathname) !== null)
    throw new ERR_INVALID_MODULE_SPECIFIER(
      resolved.pathname,
      'must not include encoded "/" or "\\" characters',
      (0, import_node_url3.fileURLToPath)(base)
    );
  const filePath = (0, import_node_url3.fileURLToPath)(resolved);
  const stats = tryStatSync(
    filePath.endsWith("/") ? filePath.slice(-1) : filePath
  );
  if (stats.isDirectory()) {
    const error = new ERR_UNSUPPORTED_DIR_IMPORT(filePath, (0, import_node_url3.fileURLToPath)(base));
    error.url = String(resolved);
    throw error;
  }
  if (!stats.isFile()) {
    throw new ERR_MODULE_NOT_FOUND(
      filePath || resolved.pathname,
      base && (0, import_node_url3.fileURLToPath)(base),
      "module"
    );
  }
  if (!preserveSymlinks) {
    const real = (0, import_node_fs2.realpathSync)(filePath);
    const { search, hash } = resolved;
    resolved = (0, import_node_url3.pathToFileURL)(real + (filePath.endsWith(import_node_path3.default.sep) ? "/" : ""));
    resolved.search = search;
    resolved.hash = hash;
  }
  return resolved;
}
function importNotDefined(specifier, packageJsonUrl, base) {
  return new ERR_PACKAGE_IMPORT_NOT_DEFINED(
    specifier,
    packageJsonUrl && (0, import_node_url3.fileURLToPath)(new import_node_url3.URL(".", packageJsonUrl)),
    (0, import_node_url3.fileURLToPath)(base)
  );
}
function exportsNotFound(subpath, packageJsonUrl, base) {
  return new ERR_PACKAGE_PATH_NOT_EXPORTED(
    (0, import_node_url3.fileURLToPath)(new import_node_url3.URL(".", packageJsonUrl)),
    subpath,
    base && (0, import_node_url3.fileURLToPath)(base)
  );
}
function throwInvalidSubpath(request, match, packageJsonUrl, internal, base) {
  const reason = `request is not a valid match in pattern "${match}" for the "${internal ? "imports" : "exports"}" resolution of ${(0, import_node_url3.fileURLToPath)(packageJsonUrl)}`;
  throw new ERR_INVALID_MODULE_SPECIFIER(
    request,
    reason,
    base && (0, import_node_url3.fileURLToPath)(base)
  );
}
function invalidPackageTarget(subpath, target, packageJsonUrl, internal, base) {
  target = typeof target === "object" && target !== null ? JSON.stringify(target, null, "") : `${target}`;
  return new ERR_INVALID_PACKAGE_TARGET(
    (0, import_node_url3.fileURLToPath)(new import_node_url3.URL(".", packageJsonUrl)),
    subpath,
    target,
    internal,
    base && (0, import_node_url3.fileURLToPath)(base)
  );
}
function resolvePackageTargetString(target, subpath, match, packageJsonUrl, base, pattern, internal, isPathMap, conditions) {
  if (subpath !== "" && !pattern && target[target.length - 1] !== "/")
    throw invalidPackageTarget(match, target, packageJsonUrl, internal, base);
  if (!target.startsWith("./")) {
    if (internal && !target.startsWith("../") && !target.startsWith("/")) {
      let isURL = false;
      try {
        new import_node_url3.URL(target);
        isURL = true;
      } catch {
      }
      if (!isURL) {
        const exportTarget = pattern ? RegExpPrototypeSymbolReplace.call(
          patternRegEx,
          target,
          () => subpath
        ) : target + subpath;
        return packageResolve(exportTarget, packageJsonUrl, conditions);
      }
    }
    throw invalidPackageTarget(match, target, packageJsonUrl, internal, base);
  }
  if (invalidSegmentRegEx.exec(target.slice(2)) !== null) {
    if (deprecatedInvalidSegmentRegEx.exec(target.slice(2)) === null) {
      if (!isPathMap) {
        const request = pattern ? match.replace("*", () => subpath) : match + subpath;
        const resolvedTarget = pattern ? RegExpPrototypeSymbolReplace.call(
          patternRegEx,
          target,
          () => subpath
        ) : target;
        emitInvalidSegmentDeprecation(
          resolvedTarget,
          request,
          match,
          packageJsonUrl,
          internal,
          base,
          true
        );
      }
    } else {
      throw invalidPackageTarget(match, target, packageJsonUrl, internal, base);
    }
  }
  const resolved = new import_node_url3.URL(target, packageJsonUrl);
  const resolvedPath = resolved.pathname;
  const packagePath = new import_node_url3.URL(".", packageJsonUrl).pathname;
  if (!resolvedPath.startsWith(packagePath))
    throw invalidPackageTarget(match, target, packageJsonUrl, internal, base);
  if (subpath === "")
    return resolved;
  if (invalidSegmentRegEx.exec(subpath) !== null) {
    const request = pattern ? match.replace("*", () => subpath) : match + subpath;
    if (deprecatedInvalidSegmentRegEx.exec(subpath) === null) {
      if (!isPathMap) {
        const resolvedTarget = pattern ? RegExpPrototypeSymbolReplace.call(
          patternRegEx,
          target,
          () => subpath
        ) : target;
        emitInvalidSegmentDeprecation(
          resolvedTarget,
          request,
          match,
          packageJsonUrl,
          internal,
          base,
          false
        );
      }
    } else {
      throwInvalidSubpath(request, match, packageJsonUrl, internal, base);
    }
  }
  if (pattern) {
    return new import_node_url3.URL(
      RegExpPrototypeSymbolReplace.call(
        patternRegEx,
        resolved.href,
        () => subpath
      )
    );
  }
  return new import_node_url3.URL(subpath, resolved);
}
function isArrayIndex(key) {
  const keyNumber = Number(key);
  if (`${keyNumber}` !== key)
    return false;
  return keyNumber >= 0 && keyNumber < 4294967295;
}
function resolvePackageTarget(packageJsonUrl, target, subpath, packageSubpath, base, pattern, internal, isPathMap, conditions) {
  if (typeof target === "string") {
    return resolvePackageTargetString(
      target,
      subpath,
      packageSubpath,
      packageJsonUrl,
      base,
      pattern,
      internal,
      isPathMap,
      conditions
    );
  }
  if (Array.isArray(target)) {
    const targetList = target;
    if (targetList.length === 0)
      return null;
    let lastException;
    let i = -1;
    while (++i < targetList.length) {
      const targetItem = targetList[i];
      let resolveResult;
      try {
        resolveResult = resolvePackageTarget(
          packageJsonUrl,
          targetItem,
          subpath,
          packageSubpath,
          base,
          pattern,
          internal,
          isPathMap,
          conditions
        );
      } catch (error) {
        const exception = (
          /** @type {ErrnoException} */
          error
        );
        lastException = exception;
        if (exception.code === "ERR_INVALID_PACKAGE_TARGET")
          continue;
        throw error;
      }
      if (resolveResult === void 0)
        continue;
      if (resolveResult === null) {
        lastException = null;
        continue;
      }
      return resolveResult;
    }
    if (lastException === void 0 || lastException === null) {
      return null;
    }
    throw lastException;
  }
  if (typeof target === "object" && target !== null) {
    const keys = Object.getOwnPropertyNames(target);
    let i = -1;
    while (++i < keys.length) {
      const key = keys[i];
      if (isArrayIndex(key)) {
        throw new ERR_INVALID_PACKAGE_CONFIG2(
          (0, import_node_url3.fileURLToPath)(packageJsonUrl),
          base,
          '"exports" cannot contain numeric property keys.'
        );
      }
    }
    i = -1;
    while (++i < keys.length) {
      const key = keys[i];
      if (key === "default" || conditions && conditions.has(key)) {
        const conditionalTarget = (
          /** @type {unknown} */
          target[key]
        );
        const resolveResult = resolvePackageTarget(
          packageJsonUrl,
          conditionalTarget,
          subpath,
          packageSubpath,
          base,
          pattern,
          internal,
          isPathMap,
          conditions
        );
        if (resolveResult === void 0)
          continue;
        return resolveResult;
      }
    }
    return null;
  }
  if (target === null) {
    return null;
  }
  throw invalidPackageTarget(
    packageSubpath,
    target,
    packageJsonUrl,
    internal,
    base
  );
}
function isConditionalExportsMainSugar(exports, packageJsonUrl, base) {
  if (typeof exports === "string" || Array.isArray(exports))
    return true;
  if (typeof exports !== "object" || exports === null)
    return false;
  const keys = Object.getOwnPropertyNames(exports);
  let isConditionalSugar = false;
  let i = 0;
  let j = -1;
  while (++j < keys.length) {
    const key = keys[j];
    const curIsConditionalSugar = key === "" || key[0] !== ".";
    if (i++ === 0) {
      isConditionalSugar = curIsConditionalSugar;
    } else if (isConditionalSugar !== curIsConditionalSugar) {
      throw new ERR_INVALID_PACKAGE_CONFIG2(
        (0, import_node_url3.fileURLToPath)(packageJsonUrl),
        base,
        `"exports" cannot contain some keys starting with '.' and some not. The exports object must either be an object of package subpath keys or an object of main entry condition name keys only.`
      );
    }
  }
  return isConditionalSugar;
}
function emitTrailingSlashPatternDeprecation(match, pjsonUrl, base) {
  const pjsonPath = (0, import_node_url3.fileURLToPath)(pjsonUrl);
  if (emittedPackageWarnings.has(pjsonPath + "|" + match))
    return;
  emittedPackageWarnings.add(pjsonPath + "|" + match);
  import_node_process2.default.emitWarning(
    `Use of deprecated trailing slash pattern mapping "${match}" in the "exports" field module resolution of the package at ${pjsonPath}${base ? ` imported from ${(0, import_node_url3.fileURLToPath)(base)}` : ""}. Mapping specifiers ending in "/" is no longer supported.`,
    "DeprecationWarning",
    "DEP0155"
  );
}
function packageExportsResolve(packageJsonUrl, packageSubpath, packageConfig, base, conditions) {
  let exports = packageConfig.exports;
  if (isConditionalExportsMainSugar(exports, packageJsonUrl, base)) {
    exports = { ".": exports };
  }
  if (own2.call(exports, packageSubpath) && !packageSubpath.includes("*") && !packageSubpath.endsWith("/")) {
    const target = exports[packageSubpath];
    const resolveResult = resolvePackageTarget(
      packageJsonUrl,
      target,
      "",
      packageSubpath,
      base,
      false,
      false,
      false,
      conditions
    );
    if (resolveResult === null || resolveResult === void 0) {
      throw exportsNotFound(packageSubpath, packageJsonUrl, base);
    }
    return resolveResult;
  }
  let bestMatch = "";
  let bestMatchSubpath = "";
  const keys = Object.getOwnPropertyNames(exports);
  let i = -1;
  while (++i < keys.length) {
    const key = keys[i];
    const patternIndex = key.indexOf("*");
    if (patternIndex !== -1 && packageSubpath.startsWith(key.slice(0, patternIndex))) {
      if (packageSubpath.endsWith("/")) {
        emitTrailingSlashPatternDeprecation(
          packageSubpath,
          packageJsonUrl,
          base
        );
      }
      const patternTrailer = key.slice(patternIndex + 1);
      if (packageSubpath.length >= key.length && packageSubpath.endsWith(patternTrailer) && patternKeyCompare(bestMatch, key) === 1 && key.lastIndexOf("*") === patternIndex) {
        bestMatch = key;
        bestMatchSubpath = packageSubpath.slice(
          patternIndex,
          packageSubpath.length - patternTrailer.length
        );
      }
    }
  }
  if (bestMatch) {
    const target = (
      /** @type {unknown} */
      exports[bestMatch]
    );
    const resolveResult = resolvePackageTarget(
      packageJsonUrl,
      target,
      bestMatchSubpath,
      bestMatch,
      base,
      true,
      false,
      packageSubpath.endsWith("/"),
      conditions
    );
    if (resolveResult === null || resolveResult === void 0) {
      throw exportsNotFound(packageSubpath, packageJsonUrl, base);
    }
    return resolveResult;
  }
  throw exportsNotFound(packageSubpath, packageJsonUrl, base);
}
function patternKeyCompare(a, b) {
  const aPatternIndex = a.indexOf("*");
  const bPatternIndex = b.indexOf("*");
  const baseLengthA = aPatternIndex === -1 ? a.length : aPatternIndex + 1;
  const baseLengthB = bPatternIndex === -1 ? b.length : bPatternIndex + 1;
  if (baseLengthA > baseLengthB)
    return -1;
  if (baseLengthB > baseLengthA)
    return 1;
  if (aPatternIndex === -1)
    return 1;
  if (bPatternIndex === -1)
    return -1;
  if (a.length > b.length)
    return -1;
  if (b.length > a.length)
    return 1;
  return 0;
}
function packageImportsResolve(name, base, conditions) {
  if (name === "#" || name.startsWith("#/") || name.endsWith("/")) {
    const reason = "is not a valid internal imports specifier name";
    throw new ERR_INVALID_MODULE_SPECIFIER(name, reason, (0, import_node_url3.fileURLToPath)(base));
  }
  let packageJsonUrl;
  const packageConfig = getPackageScopeConfig(base);
  if (packageConfig.exists) {
    packageJsonUrl = (0, import_node_url3.pathToFileURL)(packageConfig.pjsonPath);
    const imports = packageConfig.imports;
    if (imports) {
      if (own2.call(imports, name) && !name.includes("*")) {
        const resolveResult = resolvePackageTarget(
          packageJsonUrl,
          imports[name],
          "",
          name,
          base,
          false,
          true,
          false,
          conditions
        );
        if (resolveResult !== null && resolveResult !== void 0) {
          return resolveResult;
        }
      } else {
        let bestMatch = "";
        let bestMatchSubpath = "";
        const keys = Object.getOwnPropertyNames(imports);
        let i = -1;
        while (++i < keys.length) {
          const key = keys[i];
          const patternIndex = key.indexOf("*");
          if (patternIndex !== -1 && name.startsWith(key.slice(0, -1))) {
            const patternTrailer = key.slice(patternIndex + 1);
            if (name.length >= key.length && name.endsWith(patternTrailer) && patternKeyCompare(bestMatch, key) === 1 && key.lastIndexOf("*") === patternIndex) {
              bestMatch = key;
              bestMatchSubpath = name.slice(
                patternIndex,
                name.length - patternTrailer.length
              );
            }
          }
        }
        if (bestMatch) {
          const target = imports[bestMatch];
          const resolveResult = resolvePackageTarget(
            packageJsonUrl,
            target,
            bestMatchSubpath,
            bestMatch,
            base,
            true,
            true,
            false,
            conditions
          );
          if (resolveResult !== null && resolveResult !== void 0) {
            return resolveResult;
          }
        }
      }
    }
  }
  throw importNotDefined(name, packageJsonUrl, base);
}
function parsePackageName(specifier, base) {
  let separatorIndex = specifier.indexOf("/");
  let validPackageName = true;
  let isScoped = false;
  if (specifier[0] === "@") {
    isScoped = true;
    if (separatorIndex === -1 || specifier.length === 0) {
      validPackageName = false;
    } else {
      separatorIndex = specifier.indexOf("/", separatorIndex + 1);
    }
  }
  const packageName = separatorIndex === -1 ? specifier : specifier.slice(0, separatorIndex);
  if (invalidPackageNameRegEx.exec(packageName) !== null) {
    validPackageName = false;
  }
  if (!validPackageName) {
    throw new ERR_INVALID_MODULE_SPECIFIER(
      specifier,
      "is not a valid package name",
      (0, import_node_url3.fileURLToPath)(base)
    );
  }
  const packageSubpath = "." + (separatorIndex === -1 ? "" : specifier.slice(separatorIndex));
  return { packageName, packageSubpath, isScoped };
}
function packageResolve(specifier, base, conditions) {
  if (import_node_module.builtinModules.includes(specifier)) {
    return new import_node_url3.URL("node:" + specifier);
  }
  const { packageName, packageSubpath, isScoped } = parsePackageName(
    specifier,
    base
  );
  const packageConfig = getPackageScopeConfig(base);
  if (packageConfig.exists) {
    const packageJsonUrl2 = (0, import_node_url3.pathToFileURL)(packageConfig.pjsonPath);
    if (packageConfig.name === packageName && packageConfig.exports !== void 0 && packageConfig.exports !== null) {
      return packageExportsResolve(
        packageJsonUrl2,
        packageSubpath,
        packageConfig,
        base,
        conditions
      );
    }
  }
  let packageJsonUrl = new import_node_url3.URL(
    "./node_modules/" + packageName + "/package.json",
    base
  );
  let packageJsonPath = (0, import_node_url3.fileURLToPath)(packageJsonUrl);
  let lastPath;
  do {
    const stat = tryStatSync(packageJsonPath.slice(0, -13));
    if (!stat.isDirectory()) {
      lastPath = packageJsonPath;
      packageJsonUrl = new import_node_url3.URL(
        (isScoped ? "../../../../node_modules/" : "../../../node_modules/") + packageName + "/package.json",
        packageJsonUrl
      );
      packageJsonPath = (0, import_node_url3.fileURLToPath)(packageJsonUrl);
      continue;
    }
    const packageConfig2 = getPackageConfig(packageJsonPath, specifier, base);
    if (packageConfig2.exports !== void 0 && packageConfig2.exports !== null) {
      return packageExportsResolve(
        packageJsonUrl,
        packageSubpath,
        packageConfig2,
        base,
        conditions
      );
    }
    if (packageSubpath === ".") {
      return legacyMainResolve(packageJsonUrl, packageConfig2, base);
    }
    return new import_node_url3.URL(packageSubpath, packageJsonUrl);
  } while (packageJsonPath.length !== lastPath.length);
  throw new ERR_MODULE_NOT_FOUND(packageName, (0, import_node_url3.fileURLToPath)(base));
}
function isRelativeSpecifier(specifier) {
  if (specifier[0] === ".") {
    if (specifier.length === 1 || specifier[1] === "/")
      return true;
    if (specifier[1] === "." && (specifier.length === 2 || specifier[2] === "/")) {
      return true;
    }
  }
  return false;
}
function shouldBeTreatedAsRelativeOrAbsolutePath(specifier) {
  if (specifier === "")
    return false;
  if (specifier[0] === "/")
    return true;
  return isRelativeSpecifier(specifier);
}
function moduleResolve(specifier, base, conditions, preserveSymlinks) {
  const isRemote = base.protocol === "http:" || base.protocol === "https:";
  let resolved;
  if (shouldBeTreatedAsRelativeOrAbsolutePath(specifier)) {
    resolved = new import_node_url3.URL(specifier, base);
  } else if (!isRemote && specifier[0] === "#") {
    resolved = packageImportsResolve(specifier, base, conditions);
  } else {
    try {
      resolved = new import_node_url3.URL(specifier);
    } catch {
      if (!isRemote) {
        resolved = packageResolve(specifier, base, conditions);
      }
    }
  }
  (0, import_node_assert2.default)(typeof resolved !== "undefined", "expected to be defined");
  if (resolved.protocol !== "file:") {
    return resolved;
  }
  return finalizeResolution(resolved, base, preserveSymlinks);
}
function checkIfDisallowedImport(specifier, parsed, parsedParentURL) {
  if (parsed && parsedParentURL && (parsedParentURL.protocol === "http:" || parsedParentURL.protocol === "https:")) {
    if (shouldBeTreatedAsRelativeOrAbsolutePath(specifier)) {
      if (parsed && parsed.protocol !== "https:" && parsed.protocol !== "http:") {
        throw new ERR_NETWORK_IMPORT_DISALLOWED(
          specifier,
          parsedParentURL,
          "remote imports cannot import from a local location."
        );
      }
      return { url: parsed.href };
    }
    if (import_node_module.builtinModules.includes(specifier)) {
      throw new ERR_NETWORK_IMPORT_DISALLOWED(
        specifier,
        parsedParentURL,
        "remote imports cannot import from a local location."
      );
    }
    throw new ERR_NETWORK_IMPORT_DISALLOWED(
      specifier,
      parsedParentURL,
      "only relative and absolute specifiers are supported."
    );
  }
}
function throwIfUnsupportedURLProtocol(url) {
  if (url.protocol !== "file:" && url.protocol !== "data:" && url.protocol !== "node:") {
    throw new ERR_UNSUPPORTED_ESM_URL_SCHEME(url);
  }
}
function throwIfUnsupportedURLScheme(parsed, experimentalNetworkImports2) {
  if (parsed && parsed.protocol !== "file:" && parsed.protocol !== "data:" && (!experimentalNetworkImports2 || parsed.protocol !== "https:" && parsed.protocol !== "http:")) {
    throw new ERR_UNSUPPORTED_ESM_URL_SCHEME(
      parsed,
      ["file", "data"].concat(
        experimentalNetworkImports2 ? ["https", "http"] : []
      )
    );
  }
}
function defaultResolve(specifier, context = {}) {
  const { parentURL } = context;
  (0, import_node_assert2.default)(typeof parentURL !== "undefined", "expected `parentURL` to be defined");
  let parsedParentURL;
  if (parentURL) {
    try {
      parsedParentURL = new import_node_url3.URL(parentURL);
    } catch {
    }
  }
  let parsed;
  try {
    parsed = shouldBeTreatedAsRelativeOrAbsolutePath(specifier) ? new import_node_url3.URL(specifier, parsedParentURL) : new import_node_url3.URL(specifier);
    if (parsed.protocol === "data:" || experimentalNetworkImports && (parsed.protocol === "https:" || parsed.protocol === "http:")) {
      return { url: parsed.href, format: null };
    }
  } catch {
  }
  const maybeReturn = checkIfDisallowedImport(
    specifier,
    parsed,
    parsedParentURL
  );
  if (maybeReturn)
    return maybeReturn;
  if (parsed && parsed.protocol === "node:")
    return { url: specifier };
  throwIfUnsupportedURLScheme(parsed, experimentalNetworkImports);
  const conditions = getConditionsSet(context.conditions);
  const url = moduleResolve(specifier, new import_node_url3.URL(parentURL), conditions, false);
  throwIfUnsupportedURLProtocol(url);
  return {
    // Do NOT cast `url` to a string: that will work even when there are real
    // problems, silencing them
    url: url.href,
    format: defaultGetFormatWithoutErrors(url, { parentURL })
  };
}
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
  defaultResolve,
  moduleResolve
});
