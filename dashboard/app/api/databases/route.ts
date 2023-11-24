import { withRequest } from "@/utils/api";

const DELETE = withRequest(({ accessToken }) =>
  fetch(`${process.env.PROVISIONER_URL}/databases`, {
    method: "DELETE",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
  }),
);

const POST = withRequest(async ({ accessToken, req }) => {
  const apiUrl = `${process.env.PROVISIONER_URL}/databases`;
  const body = await req.json();

  console.log(body);

  return fetch(apiUrl, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
});

export { POST, DELETE };
