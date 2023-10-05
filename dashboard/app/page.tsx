import { withPageAuthRequired } from "@auth0/nextjs-auth0";
import { getSession } from "@auth0/nextjs-auth0";

const Index = async () => {
  const { user } = await getSession();

  const response = await fetch(`/api/cloud`, { method: "GET" });
  let creds = response.ok ? await response.json() : null;
  const isNewUser = !creds;

  const signupFreePlan = async () => {
    try {
      const response = await fetch("/api/cloud", { method: "POST" });
      if (!response.ok) {
        throw new Error("Server Error");
      }
    } catch (error) {
      console.error("There was a problem with the fetch operation:", error);
    }
  };

  return (
    <div className={"container"}>
      <div className={"userInfo"}>
        Welcome {user.name}! <a href="/api/auth/logout">Logout</a>
      </div>
      {isNewUser ? (
        <div className={"buttonContainer"}>
          <button className={"freePlanButton"} onClick={signupFreePlan}>
            Sign up for free plan
          </button>
        </div>
      ) : (
        <div className={"buttonContainer"}>
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

export default withPageAuthRequired(Index, { returnTo: "/" });
