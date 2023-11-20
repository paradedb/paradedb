import Stripe from "stripe";
import { NextResponse } from "next/server";

import { withStripeCustomerId } from "@/utils/api";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY ?? "");

const POST = withStripeCustomerId(async ({ id, req }) => {
  try {
    const body = await req.json();
    const paymentIntent = await stripe.paymentIntents.create({
      amount: body?.amount,
      currency: "usd",
      payment_method_types: ["card"],
      setup_future_usage: "off_session",
      customer: id,
    });

    return NextResponse.json({ clientSecret: paymentIntent.client_secret });
  } catch (err: any) {
    console.error(err);
    return NextResponse.json(err.message, { status: 500 });
  }
});

const GET = async (req: Request) => {
  try {
    const { searchParams } = new URL(req.url);
    const session_id = searchParams.get("session_id");

    if (!session_id)
      return new Response("No session_id provided", { status: 400 });

    const session = await stripe.checkout.sessions.retrieve(session_id);

    return NextResponse.json({
      status: session.status,
      customer_email: session.customer_details?.email,
    });
  } catch (err: any) {
    console.error(err);
    return NextResponse.json(err.message, { status: 500 });
  }
};

export { GET, POST };
