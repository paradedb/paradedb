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

const POST = withRequest(({ accessToken, body }) => {
  const apiUrl = `${process.env.PROVISIONER_URL}/databases`;

  console.log('Sending POST request to:', apiUrl);
  console.log('Request headers:', {
    Authorization: `Bearer ${accessToken}`,
    'Content-Type': 'application/json',
  });
  console.log('Request body:', JSON.stringify(body));

  return fetch(apiUrl, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${accessToken}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  });
});

export { POST, DELETE };
