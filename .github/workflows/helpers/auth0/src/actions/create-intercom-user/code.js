/**
 * Handler that will be called during the execution of a PostLogin flow.
 *
 * @param {Event} event - Details about the user and the context in which they are logging in.
 */
exports.onExecutePostUserRegistration = async (event) => {
  const fetch = require("node-fetch");

  fetch(`https://api.intercom.io/contacts`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Intercom-Version": "2.10",
      Authorization: `Bearer ${event.secrets.INTERCOM_ACCESS_TOKEN}}`,
    },
    body: JSON.stringify({
      email: event.user.email,
    }),
  });
};

/**
 * Handler that will be invoked when this action is resuming after an external redirect. If your
 * onExecutePostLogin function does not perform a redirect, this function can be safely ignored.
 *
 * @param {Event} event - Details about the user and the context in which they are logging in.
 */
// exports.onExecutePostUserRegistration = async (event, api) => {
// };
