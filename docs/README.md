# ParadeDB Documentation

ParadeDB [documentation](https://www.paradedb.com/docs) is built using [Mintlify](https://www.mintlify.com/docs/quickstart).

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

Same-repository pull requests that change `docs/**` create or update a Mintlify
preview deployment through the `Preview ParadeDB (Docs)` GitHub Actions
workflow. Fork pull requests do not receive previews because Mintlify cannot
preview fork-only branches from the ParadeDB repository.

## Troubleshooting

- Local preview is out of sync with the deployed documentation - Run `mint update`.
- Page loads as a 404 - Make sure you are running in the `docs/` folder that contains `docs.json`.
