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

Mintlify publishes docs changes on `main`. After its production deployment check
succeeds, `Refresh Website Docs` requests a website rebuild to refresh `/mcp`.
Preview deployments and failed or skipped checks do not trigger a rebuild.

`Publish ParadeDB (Docs)` also runs on version tags (excluding release candidates)
and manual dispatches. It asks Mintlify to rebuild its configured production
branch, waits up to 15 minutes for success, then requests the website rebuild.
`Refresh Website Docs` can be run manually to retry only the website refresh.

### Website deploy hook

Merge [website#516](https://github.com/paradedb/website/pull/516) before enabling
these workflows. In the Vercel `website` project's **Settings → Git → Deploy
Hooks**, create a hook for `main`. Store its URL as the `WEBSITE_DEPLOY_HOOK_URL`
Actions secret in `paradedb/paradedb`.

The website build downloads the published Markdown and rebuilds its MCP index.
No generated docs are committed. A successful hook request means the website
build was queued; check Vercel for its deployment result.

## Troubleshooting

- Local preview is out of sync with the deployed documentation - Run `mint update`.
- Page loads as a 404 - Make sure you are running in the `docs/` folder that contains `docs.json`.
