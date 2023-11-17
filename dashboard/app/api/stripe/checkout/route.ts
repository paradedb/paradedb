import Stripe from "stripe";
import { NextResponse } from "next/server";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY ?? "");

const POST = async () => {
  try {
    console.log("here");
    const paymentIntent = await stripe.paymentIntents.create({
      amount: 10000,
      currency: "usd",
      payment_method_types: ["card"],
    });

    return NextResponse.json({ clientSecret: paymentIntent.client_secret });
  } catch (err: any) {
    console.log(err.message);
    return NextResponse.json(err.message, { status: 500 });
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
    return NextResponse.json(err.message, { status: 500 });
  }
};

export { GET, POST };
