import { withRequest } from "@/utils/api";

const DELETE = withRequest((accessToken) =>
  fetch(`${process.env.PROVISIONER_URL}/databases`, {
    method: "DELETE",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
  }),
);

const POST = withRequest((accessToken) =>
  fetch(`${process.env.PROVISIONER_URL}/databases`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
  }),
);

export { POST, DELETE };
