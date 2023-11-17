import { ManagementClient } from "auth0";

import { withRequest } from "@/utils/api";

const GET = withRequest(({ session }) => {
  const id = session?.user.sub;

  console.log(session?.user);

  const management = new ManagementClient({
    domain: process.env.AUTH0_DOMAIN ?? "",
    clientId: process.env.AUTH0_CLIENT_ID ?? "",
    clientSecret: process.env.AUTH0_CLIENT_SECRET ?? "",
  });

  management.users.get({ id }).then(console.log);
  return null;
});

export { GET };
