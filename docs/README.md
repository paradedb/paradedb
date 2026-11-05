# ParadeDB Documentation

ParadeDB [documentation](https://www.paradedb.com/docs/start/introduction) is built using [Mintlify](https://www.mintlify.com/docs/quickstart).

## 👩‍💻 Development

Install the [Mintlify CLI](https://www.mintlify.com/docs/cli/install) to preview
documentation changes locally. The CLI requires Node.js 20.17 or later.

```bash
npm i -g mint
```

Run the following command from `docs/`, where `docs.json` is located:

```bash
mint dev
```

## 😎 Publishing Changes

Production documentation updates are deployed manually through the
`Publish ParadeDB (Docs)` GitHub Actions workflow. The workflow asks Mintlify to
rebuild the configured production branch.

## Analytics

Mintlify automatically loads `posthog.js` and `cookie-consent.css`. The script
loads both PostHog and the existing GTM container (`GTM-KMGRG564`). PostHog uses
the original docs project token and the website's consent policy: DNT/GPC skips
the SDK entirely; declining cookies uses cookieless measurement. The banner
shares the website's `paradedb_cookie_consent_v2` cookie (root path, one year),
including its legacy rejection handling, and provides a Cookie preferences
button to change the choice.

GTM is not loaded when DNT/GPC is enabled. Otherwise, Google Consent Mode defaults
are queued before GTM loads: advertising requires explicit acceptance, analytics
requires acceptance in the EEA/UK, and analytics is allowed elsewhere unless
declined. Saved choices (including legacy rejections) apply before the first tag.
Accept/Decline and saved choices from another tab update Google, HubSpot, and
PostHog together. Declining still allows Google's cookieless measurement.
Reload after changing a browser privacy signal.

Production docs use the website's `/api/analytics-consent` endpoint for regional
defaults. Failed lookups and standalone Mintlify previews default to cookieless
measurement. Keep this logic aligned with the website's `privacySignals.ts`,
`posthogConsent.ts`, and `CookieConsentBanner.tsx`.

Do not enable `integrations.posthog` or `integrations.gtm` in `docs.json` alongside
this script: the built-in integrations bypass these checks. Cookieless
capture must also be enabled in the PostHog project's settings.

Run the consent regression tests from the repository root:

```bash
node --test scripts/docs-posthog.test.cjs
```

## Troubleshooting

- Local preview is out of sync with the deployed documentation - Run `mint update`.
- Page loads as a 404 - Make sure you are running in the `docs/` folder that contains `docs.json`.
