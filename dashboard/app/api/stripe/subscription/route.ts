import jwt from "jsonwebtoken";
import Stripe from "stripe";
import { NextResponse } from "next/server";

import { getAccessToken } from "@auth0/nextjs-auth0";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY ?? "");

const GET = async () => {
  const { accessToken } = await getAccessToken();
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
  const subscriptions = await stripe.subscriptions.list({
    customer: stripeCustomerId,
    status: "active",
  });

  return NextResponse.json({ subscriptions });
};

export { GET };
