import { withPageAuthRequired } from "@auth0/nextjs-auth0";

export default withPageAuthRequired(
  async () => {
    return <div>Hello World</div>;
  },
  { returnTo: "/" },
);
