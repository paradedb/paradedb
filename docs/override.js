// Inject ParadeDB Organization structured data. Mintlify auto-emits only a
// generic WebSite node (whose creator is Mintlify), so the docs otherwise have
// no ParadeDB identity markup. Reusing the same @id as paradedb.com lets search
// engines and agents treat both sites as one Organization entity.
(function () {
  if (document.querySelector("script[data-paradedb-org]")) return;

  const organization = {
    "@context": "https://schema.org",
    "@type": "Organization",
    "@id": "https://www.paradedb.com/#organization",
    name: "ParadeDB",
    legalName: "ParadeDB, Inc.",
    url: "https://www.paradedb.com",
    logo: {
      "@type": "ImageObject",
      url: "https://www.paradedb.com/brand/paradedb-logo-light.svg",
    },
    description:
      "One Postgres for your application data, full-text search, vector retrieval, and aggregations. Home of the pg_search extension.",
    email: "hello@paradedb.com",
    contactPoint: [
      {
        "@type": "ContactPoint",
        contactType: "customer support",
        email: "support@paradedb.com",
      },
      {
        "@type": "ContactPoint",
        contactType: "sales",
        email: "sales@paradedb.com",
      },
    ],
    sameAs: [
      "https://github.com/paradedb",
      "https://x.com/paradedb",
      "https://www.linkedin.com/company/paradedb",
    ],
  };

  const script = document.createElement("script");
  script.type = "application/ld+json";
  script.setAttribute("data-paradedb-org", "");
  script.textContent = JSON.stringify(organization);
  document.head.appendChild(script);
})();

// Keep the Start flow's framework choice consistent between regular Tabs and
// CodeGroups. Mintlify reads CodeGroup choices into Tabs, but Tabs do not write
// back to CodeGroups.
(function () {
  const storageKey = "paradedb-docs-framework-tab";
  const startPaths = new Set([
    "/start/connect-your-app",
    "/start/create-your-first-index",
    "/start/run-queries",
  ]);
  const frameworks = new Set([
    "SQL",
    "Drizzle",
    "Django",
    "SQLAlchemy",
    "Rails",
    "EF Core",
  ]);
  const controlSelector = [
    '[role="tab"]',
    '[role="option"]',
    '[role="menuitem"]',
    '[role="menuitemradio"]',
    "[data-value]",
  ].join(",");

  let applyingStoredTab = false;
  let applyQueued = false;

  function isStartFlowPage() {
    const pathname = window.location.pathname
      .replace(/\/$/, "")
      .replace(/^\/docs(?=\/)/, "");

    return startPaths.has(pathname);
  }

  function normalizeLabel(value) {
    return (value || "").replace(/\s+/g, " ").trim();
  }

  function getElementFrameworkLabel(element) {
    if (!(element instanceof HTMLElement)) {
      return null;
    }

    return [
      element.getAttribute("data-value"),
      element.getAttribute("aria-label"),
      element.textContent,
    ]
      .map(normalizeLabel)
      .find((label) => frameworks.has(label));
  }

  function isVisible(element) {
    return Boolean(
      element.offsetWidth ||
      element.offsetHeight ||
      element.getClientRects().length,
    );
  }

  function isFrameworkControl(element) {
    if (!(element instanceof HTMLElement) || !isVisible(element)) {
      return false;
    }

    if (!getElementFrameworkLabel(element)) {
      return false;
    }

    const role = element.getAttribute("role");
    return (
      role === "tab" ||
      role === "option" ||
      role === "menuitem" ||
      role === "menuitemradio" ||
      element.hasAttribute("data-value") ||
      element.closest('[role="tablist"], [role="listbox"], [role="menu"]')
    );
  }

  function isSelected(element) {
    return (
      element.getAttribute("aria-selected") === "true" ||
      element.getAttribute("data-state") === "active"
    );
  }

  function findFrameworkControls(label) {
    return Array.from(document.querySelectorAll(controlSelector)).filter(
      (element) =>
        isFrameworkControl(element) &&
        getElementFrameworkLabel(element) === label,
    );
  }

  function dispatchMouseEvent(element, type, buttons) {
    element.dispatchEvent(
      new MouseEvent(type, {
        bubbles: true,
        cancelable: true,
        view: window,
        button: 0,
        buttons,
      }),
    );
  }

  function dispatchPointerEvent(element, type, buttons) {
    if (!window.PointerEvent) {
      return;
    }

    element.dispatchEvent(
      new PointerEvent(type, {
        bubbles: true,
        cancelable: true,
        view: window,
        button: 0,
        buttons,
        pointerId: 1,
        pointerType: "mouse",
        isPrimary: true,
      }),
    );
  }

  function activateFrameworkControl(element) {
    element.focus({ preventScroll: true });
    dispatchPointerEvent(element, "pointerdown", 1);
    dispatchMouseEvent(element, "mousedown", 1);
    dispatchPointerEvent(element, "pointerup", 0);
    dispatchMouseEvent(element, "mouseup", 0);
    element.click();
  }

  function getStoredFrameworkTab() {
    try {
      return window.localStorage.getItem(storageKey);
    } catch {
      return null;
    }
  }

  function setStoredFrameworkTab(label) {
    try {
      window.localStorage.setItem(storageKey, label);
    } catch {
      // Ignore storage failures so docs navigation still works normally.
    }
  }

  function rememberFrameworkTab(event) {
    if (
      applyingStoredTab ||
      !isStartFlowPage() ||
      !(event.target instanceof Element)
    ) {
      return;
    }

    const control = event.target.closest(controlSelector);
    if (!isFrameworkControl(control)) {
      return;
    }

    setStoredFrameworkTab(getElementFrameworkLabel(control));
  }

  function rememberFrameworkTabFromKeyboard(event) {
    if (event.key === "Enter" || event.key === " ") {
      rememberFrameworkTab(event);
    }
  }

  function applyStoredFrameworkTab() {
    if (applyQueued) {
      return;
    }

    applyQueued = true;
    window.requestAnimationFrame(() => {
      applyQueued = false;

      if (!isStartFlowPage()) {
        return;
      }

      const label = getStoredFrameworkTab();
      if (!frameworks.has(label)) {
        return;
      }

      const controls = findFrameworkControls(label);
      if (!controls.length || controls.some(isSelected)) {
        return;
      }

      applyingStoredTab = true;
      try {
        activateFrameworkControl(controls[0]);
      } finally {
        applyingStoredTab = false;
      }
    });
  }

  document.addEventListener("pointerdown", rememberFrameworkTab, true);
  document.addEventListener("click", rememberFrameworkTab, true);
  document.addEventListener("keydown", rememberFrameworkTabFromKeyboard, true);
  document.addEventListener("DOMContentLoaded", applyStoredFrameworkTab);
  window.addEventListener("load", applyStoredFrameworkTab);
  window.addEventListener("pageshow", applyStoredFrameworkTab);
  window.addEventListener("popstate", applyStoredFrameworkTab);

  const observer = new MutationObserver(applyStoredFrameworkTab);
  observer.observe(document.documentElement, {
    childList: true,
    subtree: true,
  });

  applyStoredFrameworkTab();
})();
