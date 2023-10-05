import { withPageAuthRequired } from "@auth0/nextjs-auth0";
import { getSession } from "@auth0/nextjs-auth0";

const Index = async () => {
  const { user, accessToken } = await getSession();
  const response = await fetch(
    `${process.env.PROVISIONER_URL}/database/credentials`,
    {
      method: "GET",
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    },
  );

  let creds = response.ok ? await response.json() : null;
  const isNewUser = !creds;

  return (
    <div>
      <div className={"userInfo"}>
        Welcome {user.name}!{" "}
        <a href={`${process.env.AUTH0_BASE_URL}/api/auth/logout`}>Logout</a>
      </div>
      {isNewUser ? (
        <div>
          {/* <button className={"freePlanButton"} onClick={signupFreePlan}>
            Sign up for free plan
          </button> */}
        </div>
      ) : (
        <div>
          {creds ? (
            <p>
              Your connection string is <br />
              host={creds.host} port={creds.port} user={creds.user} password=
              {creds.password}
            </p>
          ) : (
            <p>User already exists</p>
          )}
        </div>
      )}
    </div>
  );
};

export default withPageAuthRequired(Index, { returnTo: "/dashboard" });
