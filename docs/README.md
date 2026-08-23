# ParadeDB Documentation

ParadeDB [documentation](https://docs.paradedb.com) is built using [Mintlify](https://www.mintlify.com/docs/quickstart).

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

Changes will be deployed to production automatically after pushing to the default
branch.

You can also preview changes using PRs, which generates a preview link of the docs.

## Troubleshooting

- Local preview is out of sync with the deployed documentation - Run `mint update`.
- Page loads as a 404 - Make sure you are running in the `docs/` folder that contains `docs.json`.
