import Stripe from "stripe";
import { NextResponse } from "next/server";
import { withStripeCustomerId } from "@/utils/api";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY ?? "");

const POST = withStripeCustomerId(async ({ id, req }) => {
  const body = await req.json();
  const response = await stripe.subscriptions.create({
    customer: id,
    items: [{ price: body?.priceId }],
  });

  return NextResponse.json(response);
});

const GET = withStripeCustomerId(async ({ id }) => {
  const subscriptions = await stripe.subscriptions.list({
    customer: id,
    status: "active",
  });

  return NextResponse.json(subscriptions);
});

const PUT = withStripeCustomerId(async ({ id, req }) => {
  const body = await req.json();

  const currentSubscriptions = await stripe.subscriptions.list({
    customer: id,
    status: "active",
  });

  const oldPriceId = currentSubscriptions.data[0].items.data[0].id;

  const response = await stripe.subscriptions.update(body?.subscriptionId, {
    items: [{ id: oldPriceId, price: body?.priceId }],
  });

  return NextResponse.json(response);
});

export { GET, POST, PUT };
