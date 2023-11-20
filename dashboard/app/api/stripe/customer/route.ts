import Stripe from "stripe";

import { NextResponse } from "next/server";
import { withStripeCustomerId } from "@/utils/api";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY ?? "");

const PUT = withStripeCustomerId(async ({ id, req }) => {
  const body = await req.json();
  const response = await stripe.customers.update(id, body);

  return NextResponse.json(response);
});

export { PUT };
