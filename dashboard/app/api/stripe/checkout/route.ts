import Stripe from "stripe";
import { NextResponse } from "next/server";
import { headers } from "next/headers";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY ?? "");

const POST = async () => {
  try {
    const headersList = headers();
    const origin = headersList.get("origin");

    const session = await stripe.checkout.sessions.create({
      ui_mode: "embedded",
      line_items: [
        {
          price: "price_1OCsOWFLdqcXYNJaQLgTPBv0",
          quantity: 1,
        },
      ],
      mode: "subscription",
      return_url: `${origin}/return?session_id={CHECKOUT_SESSION_ID}`,
      automatic_tax: { enabled: true },
    });

    return NextResponse.json({ clientSecret: session.client_secret });
  } catch (err: any) {
    return new Response(err.message, { status: 500 });
  }
};

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
    return new Response(err.message, { status: 500 });
  }
};

export { GET, POST };
