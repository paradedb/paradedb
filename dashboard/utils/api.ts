import { getAccessToken, withApiAuthRequired } from "@auth0/nextjs-auth0";
import { NextResponse } from "next/server";

const withRequest = (
  customFetch: (accessToken: string) => Promise<Response>,
) => {
  return withApiAuthRequired(async () => {
    const res = new NextResponse();

    try {
      const { accessToken } = await getAccessToken();
      const response = await customFetch(accessToken ?? "");

      if (!response.ok) {
        const errorData = await response.json();
        const errorMessage = errorData.message || "An error occurred";
        return NextResponse.json(
          {
            status: response.status,
            message: errorMessage,
          },
          res,
        );
      }

      const data = await response.json();
      return NextResponse.json(data, res);
    } catch (error) {
      console.error("Failed to make the request:", error);
      return NextResponse.json(
        {
          status: 500,
          message: "Failed to connect to the server.",
        },
        res,
      );
    }
  });
};

export { withRequest };
