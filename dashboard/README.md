# ParadeDB Dashboard

This dashboard enables ParadeDB Cloud users to authenticate with, provision, and connect to their ParadeDB
instances.

The dashboard is built on Next.js using Tailwind, Tremor, and Auth0 as the primary dependencies.

## Development

1. Ensure that you have Docker installed and running.

2. Create a file called `.env.local` and paste the following content:

```ini
AUTH0_SECRET='use [openssl rand -hex 32] to generate a 32 bytes value'
STRIPE_SECRET_KEY='obtained from Stripe dashboard'
AUTH0_CLIENT_ID='obtained from Auth0 dashboard'
AUTH0_CLIENT_SECRET='obtained from Auth0 dashboard'

AUTH0_ISSUER_BASE_URL='https://paradedb-dev.us.auth0.com'
AUTH0_AUDIENCE='https://provision.cloud.paradedb.com'
AUTH0_STRIPE_CLAIM='https://paradedb.com/stripe_customer_id'
PROVISIONER_URL='https://provision-dev.cloud.paradedb.com'

NEXT_PUBLIC_INTERCOM_APP_ID='d3sdz6rs'
NEXT_PUBLIC_BASE_URL='http://localhost:3000'
NEXT_PUBLIC_POSTHOG_KEY='phc_KiWfPSoxQLmFxY5yOODDBzzP3EcyPbn9oSVtsCBbasj'
NEXT_PUBLIC_POSTHOG_HOST='https://app.posthog.com'
NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY='pk_test_51MhwqXFLdqcXYNJawYjyBo0ayjcRuRnvn0WQZmjehRfSH9gghynr0O0moJYOZMiHKMBq2aCXu9QWPLtsJPrqaQUN00At2Q5CEO'
```

Make sure to replace the necessary keys.

3. Start the development server:

```bash
docker compose up
```
