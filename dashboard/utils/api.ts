import {
  Session,
  getAccessToken,
  getSession,
  withApiAuthRequired,
} from "@auth0/nextjs-auth0";
import { NextResponse } from "next/server";

const withRequest = (
  customFetch: ({
    accessToken,
    session,
  }: {
    accessToken: string;
    session: Session;
  }) => Promise<Response>,
) => {
  return withApiAuthRequired(async () => {
    try {
      const { accessToken } = await getAccessToken();
      const session = await getSession();

      if (!accessToken || !session) {
        return NextResponse.json({
          status: 500,
          message: "No active session or access token found",
        });
      }

      const response = await customFetch({
        accessToken,
        session,
      });

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

const withAuthenticatedSession = (
  func: ({
    accessToken,
    session,
  }: {
    accessToken: string;
    session: Session;
  }) => Promise<NextResponse>,
) => {
  return withApiAuthRequired(async () => {
    console.log("hello");
    const { accessToken } = await getAccessToken();
    const session = await getSession();

    if (!accessToken || !session) {
      return NextResponse.json({
        status: 500,
        message: "No active session or access token found",
      });
    }

    return await func({ accessToken, session });
  });
};

export { withRequest, withAuthenticatedSession };
