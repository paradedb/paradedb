import { NextResponse } from "next/server";
import { getAccessToken, withApiAuthRequired } from "@auth0/nextjs-auth0";

const GET = withApiAuthRequired(async (req, res) => {
  try {
    // const res = new NextResponse();
    console.log("REQUESTINg", req, res);
    const { accessToken } = await getAccessToken();
    console.log("REQUESTED", accessToken);

    const apiKey = "Bearer " + accessToken;
    const url = `${process.env.PROVISIONER_URL}/database/credentials`;
    const provisionerResponse = await fetch(url, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
        Authorization: apiKey,
      },
    });

    // Check if the response was successful
    if (!provisionerResponse.ok) {
      // Parse error response
      const errorData = await provisionerResponse.json();
      // Return the error message from the API or use a generic message
      const errorMessage = errorData.message || "An error occurred";
      return NextResponse.json(
        { status: "error", message: errorMessage },
        { status: provisionerResponse.status },
      );
    }

    const data = await provisionerResponse.json();
    return NextResponse.json(data);
  } catch (error) {
    // Network error or other exceptions
    console.error("Failed to make the request:", error);
    return NextResponse.json(
      { status: "error", message: "Failed to connect to the server." },
      { status: 500 },
    ); // 500 for Internal Server Error
  }
});

export { GET };

export async function POST(req) {
  try {
    const res = new NextResponse();
    const { accessToken } = await getAccessToken(req, res);

    const apiKey = "Bearer " + accessToken;
    const url = `${process.env.PROVISIONER_URL}/database/provision`;
    const provisionerResponse = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: apiKey,
      },
    });

    // Check if the response was successful
    if (!provisionerResponse.ok) {
      // Parse error response
      const errorData = await provisionerResponse.json();
      // Return the error message from the API or use a generic message
      const errorMessage = errorData.message || "An error occurred";
      return NextResponse.json(
        { status: "error", message: errorMessage },
        { status: provisionerResponse.status },
      );
    }
    const data = await provisionerResponse.json();
    return NextResponse.json(data);
  } catch (error) {
    // Network error or other exceptions
    console.error("Failed to make the request:", error);
    return NextResponse.json(
      { status: "error", message: "Failed to connect to the server." },
      { status: 500 },
    ); // 500 for Internal Server Error
  }
}
