## ParadeDB Dashboard

This dashboard enables ParadeDB Cloud users to authenticate with, provision, and connect to their ParadeDB
instances.

The dashboard is built on Next.js using Tailwind, Tremor, and Auth0 as the primary dependencies.

## Development

1. From your terminal, install the project's dependencies:

```bash
yarn install
```

2. Create a file called `.env.local` and paste the following content:

```
AUTH0_SECRET='use [openssl rand -hex 32] to generate a 32 bytes value'
AUTH0_BASE_URL='http://localhost:3000'
AUTH0_ISSUER_BASE_URL='https://paradedb-dev.us.auth0.com'
AUTH0_CLIENT_ID='v5XiG0Qe2xDE91bEELuzGGibWCIwUWnb'
AUTH0_CLIENT_SECRET='9NKYhrRoYfgYLc_AxZ_19MuZpDJtUJa1ZP4sSVf0NsHqX-BQhU_ScBKCqXh_wOjB'
```

Make sure to replace `AUTH0_SECRET` with the output of `openssl rand -hex 32`.

3. Start the development server:

```bash
yarn dev
```
