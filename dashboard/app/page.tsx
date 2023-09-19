import { withPageAuthRequired } from '@auth0/nextjs-auth0';

export default withPageAuthRequired(async function Home() {
  return <div>Hello World</div>
}, { returnTo: '/' })