import { getAccessToken, withApiAuthRequired } from "@auth0/nextjs-auth0";
import { NextResponse } from "next/server";

// Tmp comment for testing
const withRequest = (
  customFetch: (accessToken: string) => Promise<Response>,
) => {
  return withApiAuthRequired(async () => {
    try {
      const { accessToken } = await getAccessToken();
      const response = await customFetch(accessToken ?? "");

      if (!response.ok) {
        const errorData = await response.json();
        const errorMessage = errorData.message || "An error occurred";
        return NextResponse.json({
          status: response.status,
          message: errorMessage,
        });
      }

      const data = await response.json();
      return NextResponse.json(data);
    } catch (error: any) {
      return NextResponse.json({
        status: 500,
        message: error?.code,
      });
    }
  });
};

export { withRequest };
