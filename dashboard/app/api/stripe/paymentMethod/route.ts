import Stripe from "stripe";

import { withStripeCustomerId } from "@/utils/api";
import { NextResponse } from "next/server";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY ?? "");

const GET = withStripeCustomerId(async ({ id }) => {
  const paymentMethods = await stripe.customers.listPaymentMethods(id, {
    type: "card",
  });

  return NextResponse.json(paymentMethods);
});

const DELETE = withStripeCustomerId(async ({ req }) => {
  const body = await req.json();
  const response = await stripe.paymentMethods.detach(body?.paymentMethodId);

  return NextResponse.json(response);
});

export { GET, DELETE };
