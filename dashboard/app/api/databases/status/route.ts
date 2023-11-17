import { withRequest } from "@/utils/api";

const GET = withRequest(({ accessToken }) =>
  fetch(`${process.env.PROVISIONER_URL}/databases/status`, {
    method: "GET",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
  }),
);

export { GET };
