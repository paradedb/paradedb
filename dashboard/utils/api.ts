import { getAccessToken, withApiAuthRequired } from "@auth0/nextjs-auth0";
import { NextResponse } from "next/server";

const withRequest = (
  customFetch: (accessToken: string) => Promise<Response>,
) => {
  return withApiAuthRequired(async (req: Request) => {
    try {
      const { accessToken } = await getAccessToken();
      const response = await customFetch(accessToken ?? "");

      console.log(response);

      if (!response.ok) {
        const errorData = await response.json();
        const errorMessage = errorData.message || "An error occurred";
        return NextResponse.json(
          {
            status: "error",
            message: errorMessage,
          },
          { status: response.status },
        );
      }

      const data = await response.json();

      const res = new NextResponse();
      return NextResponse.json(data, res);
    } catch (error) {
      console.error("Failed to make the request:", error);
      return NextResponse.json(
        {
          status: "error",
          message: "Failed to connect to the server.",
        },
        { status: 500 },
      );
    }
  });
};

export { withRequest };
