import { withRequest } from "@/utils/api";

const GET = withRequest((accessToken: string) =>
  fetch(`${process.env.PROVISIONER_URL}/databases/credentials`, {
    method: "GET",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
  }),
);

export { GET };
