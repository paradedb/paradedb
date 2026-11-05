const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { test } = require("node:test");

const source = fs.readFileSync(
  path.join(__dirname, "../docs/posthog.js"),
  "utf8",
);
const settle = () => new Promise((resolve) => setImmediate(resolve));

function browser(options = {}) {
  const calls = [];
  const elements = [];
  const listeners = {};
  const storage = new Map([["paradedb_posthog_consent", "1"]]);
  function element(tag) {
    const node = {
      tag,
      hidden: false,
      dataset: {},
      listeners: {},
      setAttribute() {},
      focus() {},
      remove() {},
      addEventListener(name, callback) {
        this.listeners[name] = callback;
      },
      querySelectorAll() {
        return this.buttons;
      },
      querySelector() {
        return this.buttons[0];
      },
      set innerHTML(value) {
        this.buttons = [false, true].map((allowed) => {
          const button = element("button");
          button.dataset.consent = String(allowed);
          return button;
        });
      },
    };
    elements.push(node);
    return node;
  }
  const document = {
    cookie: options.cookie ?? "",
    readyState: "complete",
    createElement: element,
    head: { appendChild: (script) => calls.push({ type: "script", script }) },
    body: { append() {} },
  };
  const sdk = {
    init: (key, config) =>
      calls.push({
        type: "init",
        key,
        config,
        consent: storage.get(config.consent_persistence_name),
      }),
    opt_in_capturing: () => calls.push({ type: "accept" }),
    opt_out_capturing: () => calls.push({ type: "decline" }),
  };
  const window = {
    doNotTrack: options.legacyDnt,
    addEventListener: (name, callback) => (listeners[name] = callback),
  };
  const globals = {
    window,
    document,
    navigator: options.navigator ?? {},
    location: { hostname: options.hostname ?? "www.paradedb.com" },
    AbortSignal,
    localStorage: {
      setItem(key, value) {
        if (options.storageBlocked) throw new Error("blocked");
        storage.set(key, value);
      },
    },
    fetch: async () => {
      calls.push({ type: "lookup" });
      return options.lookup
        ? options.lookup()
        : { ok: true, json: async () => ({ analyticsCookiesAllowed: false }) };
    },
  };
  vm.runInNewContext(source, globals);
  return {
    calls,
    elements,
    globals,
    listeners,
    storage,
    runAgain: () => vm.runInNewContext(source, globals),
    loadSDK() {
      window.posthog = sdk;
      calls.find((call) => call.type === "script").script.onload();
    },
    choose(allowed) {
      elements
        .find((node) => node.id === "paradedb-cookie-banner")
        .buttons[allowed ? 1 : 0].listeners.click();
    },
  };
}

test("DNT and GPC suppress the SDK download, lookup, storage writes and banner", async () => {
  for (const options of [
    { navigator: { doNotTrack: "1" } },
    { navigator: { doNotTrack: "yes" } },
    { navigator: { msDoNotTrack: "1" } },
    { navigator: { globalPrivacyControl: true } },
    { legacyDnt: "1" },
  ]) {
    const b = browser({
      ...options,
      cookie: "paradedb_cookie_consent_v2=true",
    });
    await settle();
    assert.deepEqual(b.calls, []);
    assert.deepEqual(b.elements, []);
  }
});

test("saved consent replaces stale SDK permission before the first event", async () => {
  for (const [cookie, expected] of [
    ["paradedb_cookie_consent_v2=true", "1"],
    ["paradedb_cookie_consent_v2=false", "0"],
    ["paradedb_cookie_consent=false", "0"],
    ["paradedb_cookie_consent=false; paradedb_cookie_consent_v2=true", "1"],
  ]) {
    const b = browser({ cookie });
    b.loadSDK();
    const init = b.calls.at(-1);
    assert.equal(init.type, "init");
    assert.equal(init.consent, expected);
    assert.equal(init.config.cookieless_mode, "on_reject");
    assert.equal(init.config.respect_dnt, true);
    assert.equal(init.config.opt_out_persistence_by_default, true);
    assert.equal(
      b.calls.some((call) => call.type === "lookup"),
      false,
    );
    b.globals.navigator.globalPrivacyControl = true;
    assert.equal(init.config.before_send({ event: "$pageview" }), null);
  }
});

test("regional defaults, failed lookups and standalone previews", async () => {
  for (const [options, expected] of [
    [
      {
        lookup: async () => ({
          ok: true,
          json: async () => ({ analyticsCookiesAllowed: true }),
        }),
      },
      "1",
    ],
    [{}, "0"],
    [
      {
        lookup: async () => {
          throw new Error("offline");
        },
      },
      "0",
    ],
    [{ lookup: async () => ({ ok: false }) }, "0"],
    [{ hostname: "paradedb-dev.mintlify.app" }, "0"],
  ]) {
    const b = browser(options);
    await settle();
    b.loadSDK();
    assert.equal(b.calls.at(-1).consent, expected);
    if (options.hostname) assert.equal(b.calls[0].type, "script");
  }
});

test("a banner choice wins over pending region and SDK requests", async () => {
  for (const allowed of [true, false]) {
    let resolve;
    const pending = new Promise((done) => {
      resolve = done;
    });
    const b = browser({ lookup: () => pending });
    b.choose(allowed);
    assert.equal(
      b.globals.window.dataLayer.at(-1)[2].analytics_storage,
      allowed ? "granted" : "denied",
    );
    resolve({
      ok: true,
      json: async () => ({ analyticsCookiesAllowed: !allowed }),
    });
    await settle();
    b.loadSDK();
    assert.equal(b.calls.at(-1).consent, allowed ? "1" : "0");
    assert.match(
      b.globals.document.cookie,
      /Path=\/; Max-Age=31536000; SameSite=Lax/,
    );
    assert.equal(b.calls.filter((call) => call.type === "script").length, 1);
    b.choose(!allowed);
    assert.equal(b.calls.at(-1).type, allowed ? "decline" : "accept");
    b.runAgain();
    assert.equal(b.calls.filter((call) => call.type === "init").length, 1);
  }
});

test("privacy enabled during SDK loading prevents initialization", async () => {
  const b = browser({ cookie: "paradedb_cookie_consent_v2=true" });
  b.globals.navigator.globalPrivacyControl = true;
  b.loadSDK();
  assert.equal(
    b.calls.some((call) => call.type === "init"),
    false,
  );
});

test("blocked storage prevents SDK loading; another tab's choice is honored on focus", async () => {
  const blocked = browser({
    storageBlocked: true,
    cookie: "paradedb_cookie_consent_v2=true",
  });
  assert.deepEqual(blocked.calls, []);
  const b = browser({ cookie: "paradedb_cookie_consent_v2=true" });
  b.loadSDK();
  b.globals.document.cookie = "paradedb_cookie_consent_v2=false";
  b.listeners.focus();
  assert.equal(b.calls.at(-1).type, "decline");
  b.elements
    .find((node) => node.id === "paradedb-cookie-preferences")
    .listeners.click();
  assert.equal(
    b.elements.find((node) => node.id === "paradedb-cookie-banner").hidden,
    false,
  );
});
