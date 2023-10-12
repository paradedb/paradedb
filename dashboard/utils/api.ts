import { getAccessToken, withApiAuthRequired } from "@auth0/nextjs-auth0";
import { NextResponse } from "next/server";

const withRequest = (
  customFetch: (accessToken: string) => Promise<Response>,
) => {
  return withApiAuthRequired(async (req) => {
    const res = new NextResponse();

    try {
      const { accessToken } = await getAccessToken(req, res);
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
    } catch (error: any) {
      return NextResponse.json(
        {
          status: 500,
          message: error?.code,
        },
        res,
      );
    }
  });
};

export { withRequest };
