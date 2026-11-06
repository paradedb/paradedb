// Mintlify loads this on every page; the website hosts the shared consent code.
(function () {
  if (window.__paradedbDocsConsentLoading) return;
  window.__paradedbDocsConsentLoading = true;

  function initialize() {
    const shared = window.ParadeDBConsent;
    if (shared?.version !== 1) return;
    const consent = shared.create({
      gtmId: "GTM-KMGRG564",
      posthogKey: "phc_KiWfPSoxQLmFxY5yOODDBzzP3EcyPbn9oSVtsCBbasj",
      posthogHost: "https://us.i.posthog.com",
      // Previews don't have the website's country endpoint.
      regionEndpoint: ["www.paradedb.com", "paradedb.com"].includes(
        location.hostname,
      )
        ? "/api/analytics-consent"
        : null,
    });
    void consent.start();
    if (consent.snapshot().blocked) return;

    const banner = document.createElement("section");
    banner.id = "paradedb-cookie-banner";
    banner.setAttribute("aria-label", "Cookie preferences");
    banner.innerHTML = `
      <p>We use cookies for analytics and personalized ads. Declining limits
      tracking to cookieless measurement.
      <a href="https://www.paradedb.com/legal/privacy">Privacy Policy</a></p>
      <div><button type="button" data-consent="false">Decline</button>
      <button type="button" data-consent="true">Accept</button></div>`;
    const preferences = document.createElement("button");
    preferences.id = "paradedb-cookie-preferences";
    preferences.type = "button";
    preferences.textContent = "Cookie preferences";
    preferences.setAttribute("aria-controls", banner.id);

    function showBanner(show, focus = false) {
      banner.hidden = !show;
      preferences.hidden = show;
      if (focus) (show ? banner.querySelector("button") : preferences).focus();
    }
    banner.querySelectorAll("button").forEach((button) => {
      button.addEventListener("click", () => {
        consent.setConsent(button.dataset.consent === "true");
        showBanner(false, true);
      });
    });
    function openPreferences() {
      if (!consent.snapshot().blocked) showBanner(true, true);
    }
    preferences.addEventListener("click", openPreferences);
    window.addEventListener("paradedb:cookie-preferences", openPreferences);
    document.body.append(banner, preferences);
    consent.subscribe(({ blocked, saved }) => {
      if (blocked) {
        banner.hidden = true;
        preferences.hidden = true;
      } else showBanner(saved === undefined);
    });
  }

  function ready() {
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", initialize, { once: true });
    } else initialize();
  }
  if (window.ParadeDBConsent?.version === 1) ready();
  else {
    const script = document.createElement("script");
    script.src = "https://www.paradedb.com/analytics-consent-v1.js";
    script.async = true;
    script.onload = ready;
    script.onerror = () => {
      // Allow a later page load to retry.
      window.__paradedbDocsConsentLoading = false;
      script.remove();
    };
    document.head.appendChild(script);
  }
})();
