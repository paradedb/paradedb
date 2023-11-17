/**
 * Handler that will be called during the execution of a PostLogin flow.
 *
 * @param {Event} event - Details about the user and the context in which they are logging in.
 * @param {PostLoginAPI} api - Interface whose methods can be used to change the behavior of the login.
 */
exports.onExecutePostLogin = async (event, api) => {
  const stripe = require("stripe")(event.secrets.STRIPE_API_KEY);

  if (!event.user.app_metadata.stripeCustomerId) {
    try {
      const customer = await stripe.customers.create({
        email: event.user.email,
        name: event.user.name,
      });

      api.user.setAppMetadata("stripeCustomerId", customer.id);
      api.accessToken.setCustomClaim(
        `https://paradedb.com/stripe_customer_id`,
        customer.id,
      );
    } catch (error) {
      console.log("Error creating Stripe customer:", error);
    }
  }
};

/**
 * Handler that will be invoked when this action is resuming after an external redirect. If your
 * onExecutePostLogin function does not perform a redirect, this function can be safely ignored.
 *
 * @param {Event} event - Details about the user and the context in which they are logging in.
 * @param {PostLoginAPI} api - Interface whose methods can be used to change the behavior of the login.
 */
// exports.onContinuePostLogin = async (event, api) => {
// };
