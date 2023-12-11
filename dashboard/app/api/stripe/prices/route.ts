import Stripe from "stripe";
import { NextResponse } from "next/server";

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY ?? "");

const GET = async (req: Request) => {
  try {
    const { searchParams } = new URL(req.url);
    const prices = await stripe.prices.list(
      searchParams as Stripe.RequestOptions,
    );
    return NextResponse.json(prices);
  } catch (err: any) {
    return new Response(err.message, { status: 500 });
  }
};

export { GET };
