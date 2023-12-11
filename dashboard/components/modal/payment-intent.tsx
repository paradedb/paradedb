"use client";

import { useState, useEffect, Fragment, FormEvent } from "react";
import { Dialog, Transition } from "@headlessui/react";
import { Button } from "@tremor/react";
import { loadStripe } from "@stripe/stripe-js";
import {
  Elements,
  PaymentElement,
  useStripe,
  useElements,
} from "@stripe/react-stripe-js";
import { XMarkIcon } from "@heroicons/react/24/outline";
import { PrimaryButton } from "@/components/tremor";

import { useAppState } from "@/components/context";
import { GENERIC_ERROR } from ".";

const stripePromise = loadStripe(
  process.env.NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY ?? "",
);

interface PaymentIntentModalProps {
  isOpen: boolean;
  onClose: () => void;
  amount: number;
}

interface EmbeddedPaymentFormProps {
  onClose: () => void;
}

const EmbeddedPaymentForm = ({ onClose }: EmbeddedPaymentFormProps) => {
  const stripe = useStripe();
  const elements = useElements();
  const [isLoading, setIsLoading] = useState(false);
  const { setNotification } = useAppState();

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();

    setIsLoading(true);

    if (!stripe || !elements) {
      setIsLoading(false);
      setNotification?.(GENERIC_ERROR);
      return;
    }

    const paymentIntentResult = await stripe.confirmPayment({
      elements,
      confirmParams: {
        return_url: process.env.NEXT_PUBLIC_BASE_URL ?? "",
      },
      redirect: "if_required",
    });

    if (paymentIntentResult.error) {
      setIsLoading(false);
      setNotification?.(GENERIC_ERROR);
      return;
    }

    const updateCustomerResult = await fetch("/api/stripe/customer", {
      method: "PUT",
      body: JSON.stringify({
        invoice_settings: {
          default_payment_method:
            paymentIntentResult.paymentIntent?.payment_method,
        },
      }),
    });

    if (updateCustomerResult.status !== 200) {
      setIsLoading(false);
      setNotification?.(GENERIC_ERROR);
      return;
    }

    setIsLoading(false);
    onClose();
  };

  useEffect(() => {
    if (!stripe) return;

    const clientSecret = new URLSearchParams(window.location.search).get(
      "payment_intent_client_secret",
    );

    if (!clientSecret) return;

    stripe.retrievePaymentIntent(clientSecret).then((intent) => {
      if (intent.error) setNotification?.(GENERIC_ERROR);
    });
  }, [stripe]);

  return (
    <form onSubmit={handleSubmit}>
      <PaymentElement />
      <PrimaryButton
        className="rounded py-3 mt-6 w-full bg-indigo-500 text-gray-100 border-0 hover:bg-indigo-400 hover:text-gray-100 hover:border-0 duration-100"
        type="submit"
        loading={isLoading}
        disabled={!stripe || !elements}
      >
        Add Payment
      </PrimaryButton>
    </form>
  );
};

const PaymentIntentModal = ({
  isOpen,
  onClose,
  amount,
}: PaymentIntentModalProps) => {
  const [clientSecret, setClientSecret] = useState("");

  const appearance = {
    theme: "flat" as any,
  };
  const options = {
    clientSecret,
    appearance,
  };

  useEffect(() => {
    if (amount <= 0) return;

    fetch("/api/stripe/paymentIntent", {
      method: "POST",
      body: JSON.stringify({
        amount,
      }),
    })
      .then((res) => res.json())
      .then((data) => {
        setClientSecret(data.clientSecret);
      });
  }, [amount]);

  return (
    <Transition appear show={isOpen} as={Fragment}>
      <Dialog as="div" className="relative z-20" onClose={() => {}}>
        <Transition.Child
          as={Fragment}
          enter="ease-out duration-300"
          enterFrom="opacity-0"
          enterTo="opacity-100"
          leave="ease-in duration-200"
          leaveFrom="opacity-100"
          leaveTo="opacity-0"
        >
          <div className="fixed inset-0 bg-black/25" />
        </Transition.Child>

        <div className="fixed inset-0 overflow-y-auto">
          <div className="flex min-h-full items-center justify-center p-4 text-center">
            <Transition.Child
              as={Fragment}
              enter="ease-out duration-300"
              enterFrom="opacity-0 scale-95"
              enterTo="opacity-100 scale-100"
              leave="ease-in duration-200"
              leaveFrom="opacity-100 scale-100"
              leaveTo="opacity-0 scale-95"
            >
              <Dialog.Panel className="bg-neutral-200 border-0 transform overflow-y-scroll scrollbar-hidden rounded-lg p-12 text-left align-middle transition-all">
                <Button
                  icon={XMarkIcon}
                  variant="light"
                  color={"neutral"}
                  size="xl"
                  onClick={onClose}
                  className="fixed top-12 right-12"
                />
                <div className="w-full mt-12">
                  {clientSecret && (
                    <Elements stripe={stripePromise} options={options}>
                      <EmbeddedPaymentForm onClose={onClose} />
                    </Elements>
                  )}
                </div>
              </Dialog.Panel>
            </Transition.Child>
          </div>
        </div>
      </Dialog>
    </Transition>
  );
};

export { PaymentIntentModal };
