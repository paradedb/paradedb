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

Mintlify automatically loads `posthog.js` and `cookie-consent.css`. PostHog uses
the original docs project token and the website's consent policy: DNT/GPC skips
the SDK entirely; declining cookies uses cookieless measurement. The banner
shares the website's `paradedb_cookie_consent_v2` cookie (root path, one year),
including its legacy rejection handling, and provides a Cookie preferences
button to change the choice.

Production docs use the website's `/api/analytics-consent` endpoint for regional
defaults. Failed lookups and standalone Mintlify previews default to cookieless
measurement. Keep this logic aligned with the website's `privacySignals.ts`,
`posthogConsent.ts`, and `CookieConsentBanner.tsx`.

Do not enable `integrations.posthog` in `docs.json` alongside this script: the
built-in integration cannot apply these checks before initialization. Cookieless
capture must also be enabled in the PostHog project's settings.

Run the consent regression tests from the repository root:

```bash
node --test scripts/docs-posthog.test.cjs
```

## Troubleshooting

- Local preview is out of sync with the deployed documentation - Run `mint update`.
- Page loads as a 404 - Make sure you are running in the `docs/` folder that contains `docs.json`.
