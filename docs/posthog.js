// Mintlify loads this file on every page. Keep the consent policy aligned with
// website/src/lib/{privacySignals,posthogConsent}.ts and CookieConsentBanner.tsx.
// Do not enable integrations.posthog or integrations.gtm: they bypass these checks.
(function () {
  if (window.__paradedbDocsPostHog) return;
  window.__paradedbDocsPostHog = true;

  const cookieName = "paradedb_cookie_consent_v2";
  const consentStorageName = "paradedb_posthog_consent";
  const gtmId = "GTM-KMGRG564";
  // Same EEA/UK override as website/src/lib/privacySignals.ts.
  const analyticsOptInCountries = [
    "AT",
    "BE",
    "BG",
    "HR",
    "CY",
    "CZ",
    "DK",
    "EE",
    "FI",
    "FR",
    "DE",
    "GR",
    "HU",
    "IE",
    "IT",
    "LV",
    "LT",
    "LU",
    "MT",
    "NL",
    "PL",
    "PT",
    "RO",
    "SK",
    "SI",
    "ES",
    "SE",
    "GB",
    "IS",
    "LI",
    "NO",
  ];
  // Public project token from the original Mintlify integration.
  const projectKey = "phc_KiWfPSoxQLmFxY5yOODDBzzP3EcyPbn9oSVtsCBbasj";
  let initialized = false;
  let loading = false;
  let explicitChoice;
  let regionalDefault = false;
  let appliedConsent;

  function browserRequestsPrivacy() {
    return (
      navigator.globalPrivacyControl === true ||
      [navigator.doNotTrack, navigator.msDoNotTrack, window.doNotTrack].some(
        (value) => value === "1" || value === "yes",
      )
    );
  }

  function savedAnalyticsConsent() {
    const cookies = document.cookie.split(";").map((part) => part.trim());
    const saved = cookies.find((part) => part.startsWith(`${cookieName}=`));
    if (saved === `${cookieName}=true`) return true;
    if (saved === `${cookieName}=false`) return false;
    if (cookies.includes("paradedb_cookie_consent=false")) return false;
    return undefined;
  }

  function initializeTagManager() {
    if (browserRequestsPrivacy()) return;
    const saved = savedAnalyticsConsent();
    window.dataLayer = window.dataLayer || [];
    window.gtag = function () {
      window.dataLayer.push(arguments);
    };
    const advertisingConsent = saved === true ? "granted" : "denied";
    // Queue defaults before gtm.js, including on a return visit with a saved
    // rejection. Google resolves the region, just as on the main website.
    window.gtag("consent", "default", {
      analytics_storage: saved === false ? "denied" : "granted",
      ad_storage: advertisingConsent,
      ad_user_data: advertisingConsent,
      ad_personalization: advertisingConsent,
    });
    if (saved !== true) {
      window.gtag("consent", "default", {
        analytics_storage: "denied",
        wait_for_update: 500,
        region: analyticsOptInCountries,
      });
    }
    if (saved !== undefined) updateTagConsent(saved);
    window.dataLayer.push({ "gtm.start": Date.now(), event: "gtm.js" });
    const script = document.createElement("script");
    script.src = `https://www.googletagmanager.com/gtm.js?id=${gtmId}`;
    script.async = true;
    document.head.appendChild(script);
  }

  function updateTagConsent(granted) {
    const hubspot = (window._hsq = window._hsq || []);
    hubspot.push(granted ? ["doNotTrack", { track: true }] : ["doNotTrack"]);
    const consent = granted ? "granted" : "denied";
    window.gtag("consent", "update", {
      analytics_storage: consent,
      ad_storage: consent,
      ad_user_data: consent,
      ad_personalization: consent,
    });
  }

  function applyConsent() {
    if (browserRequestsPrivacy()) return;
    const allowed =
      explicitChoice ?? savedAnalyticsConsent() ?? regionalDefault;

    if (!initialized) {
      // Mirror the current choice before SDK initialization, replacing stale
      // PostHog permission. Storage failure must never grant cookie permission.
      try {
        localStorage.setItem(consentStorageName, allowed ? "1" : "0");
      } catch {
        return;
      }
      if (!window.posthog?.init) {
        if (loading) return;
        loading = true;
        const script = document.createElement("script");
        script.src = "https://us-assets.i.posthog.com/static/array.js";
        script.async = true;
        // Re-read both privacy signals and consent after the download: a click
        // during loading must win over the earlier regional/saved default.
        script.onload = () => {
          loading = false;
          if (window.posthog?.init) applyConsent();
        };
        script.onerror = () => {
          loading = false;
          script.remove();
        };
        document.head.appendChild(script);
        return;
      }
      initialized = true;
      window.posthog.init(projectKey, {
        api_host: "https://us.i.posthog.com",
        defaults: "2026-08-30",
        capture_exceptions: true,
        respect_dnt: true,
        cookieless_mode: "on_reject",
        opt_out_capturing_by_default: true,
        opt_out_persistence_by_default: true,
        consent_persistence_name: consentStorageName,
        opt_out_capturing_persistence_type: "localStorage",
        before_send: (event) => (browserRequestsPrivacy() ? null : event),
      });
    } else if (allowed !== appliedConsent) {
      if (allowed) {
        window.posthog.opt_in_capturing({ captureEventName: false });
      } else {
        window.posthog.opt_out_capturing();
      }
    }
    appliedConsent = allowed;
  }

  function updateConsent(granted) {
    if (browserRequestsPrivacy()) return;
    explicitChoice = granted;
    // Same host, root path, lifetime, and SameSite setting as the website.
    document.cookie = `${cookieName}=${granted}; Path=/; Max-Age=31536000; SameSite=Lax`;
    updateTagConsent(granted);
    applyConsent();
  }

  function mountPreferences() {
    if (browserRequestsPrivacy()) return;
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
        updateConsent(button.dataset.consent === "true");
        showBanner(false, true);
      });
    });
    function openPreferences() {
      if (!browserRequestsPrivacy()) showBanner(true, true);
    }
    preferences.addEventListener("click", openPreferences);
    window.addEventListener("paradedb:cookie-preferences", openPreferences);
    // Pick up a cookie choice made in another website/docs tab.
    window.addEventListener("focus", () => {
      if (browserRequestsPrivacy()) {
        updateTagConsent(false);
        banner.hidden = true;
        preferences.hidden = true;
        return;
      }
      explicitChoice = undefined;
      const saved = savedAnalyticsConsent();
      if (saved !== undefined) updateTagConsent(saved);
      applyConsent();
      showBanner(savedAnalyticsConsent() === undefined);
    });
    document.body.append(banner, preferences);
    showBanner(savedAnalyticsConsent() === undefined);
  }

  async function initialize() {
    if (browserRequestsPrivacy()) return;
    initializeTagManager();
    mountPreferences();
    if (savedAnalyticsConsent() === undefined) {
      // The website serves this endpoint outside the /docs Mintlify proxy.
      // Standalone Mintlify previews have no country endpoint: stay cookieless.
      if (["www.paradedb.com", "paradedb.com"].includes(location.hostname)) {
        try {
          const response = await fetch("/api/analytics-consent", {
            cache: "no-store",
            signal: AbortSignal.timeout(3000),
          });
          if (response.ok) {
            const result = await response.json();
            regionalDefault = result.analyticsCookiesAllowed === true;
          }
        } catch {
          // Unknown region or failed lookup stays cookieless.
        }
      }
    }
    applyConsent();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initialize, { once: true });
  } else {
    void initialize();
  }
})();
