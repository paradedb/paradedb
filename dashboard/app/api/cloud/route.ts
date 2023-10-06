import { getAccessToken, withApiAuthRequired } from "@auth0/nextjs-auth0";
import { NextApiRequest, NextApiResponse } from "next";
import { NextResponse } from "next/server";

const POST = withApiAuthRequired(
  async (req: NextApiRequest, res: NextApiResponse) => {
    try {
      const { accessToken } = await getAccessToken(req, res);

      const apiKey = `Bearer ${accessToken}`;
      const url = `${process.env.PROVISIONER_URL}/database/provision`;
      const provisionerResponse = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: apiKey,
        },
      });

      if (!provisionerResponse.ok) {
        const errorData = await provisionerResponse.json();
        const errorMessage = errorData.message || "An error occurred";
        return NextResponse.json(
          { status: "error", message: errorMessage },
          { status: provisionerResponse.status },
        );
      }

      const data = await provisionerResponse.json();
      return NextResponse.json(data);
    } catch (error) {
      console.error("Failed to make the request:", error);
      return NextResponse.json(
        { status: "error", message: "Failed to connect to the server." },
        { status: 500 },
      );
    }
  },
);

export { POST };
