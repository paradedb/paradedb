import jwt from "jsonwebtoken";
import {
  Session,
  getAccessToken,
  getSession,
  withApiAuthRequired,
} from "@auth0/nextjs-auth0";
import { NextRequest, NextResponse } from "next/server";

const withRequest = (
  customFetch: ({
    accessToken,
    session,
    body,
  }: {
    accessToken: string;
    session: Session;
    body: string;
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
        body,
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

const withStripeCustomerId = (
  func: ({
    id,
    req,
  }: {
    id: string;
    req: NextRequest;
  }) => Promise<NextResponse>,
) => {
  return withApiAuthRequired(async (req: NextRequest) => {
    const { accessToken } = await getAccessToken();

    if (!accessToken) {
      return NextResponse.json({
        status: 500,
        message: "No access token found",
      });
    }

    const decoded = jwt.decode(accessToken ?? "") as jwt.JwtPayload;

    if (!decoded)
      return NextResponse.json({
        status: 500,
        message: "Access token could not be decoded",
      });

    if (!process.env.AUTH0_STRIPE_CLAIM)
      return NextResponse.json({
        status: 500,
        message: "AUTH0_STRIPE_CLAIM not set",
      });

    const stripeCustomerId = decoded[process.env.AUTH0_STRIPE_CLAIM];

    if (!stripeCustomerId)
      return NextResponse.json({
        status: 500,
        message: "Stripe customer ID not found",
      });

    return await func({ id: stripeCustomerId, req });
  });
};

export { withRequest, withStripeCustomerId };
