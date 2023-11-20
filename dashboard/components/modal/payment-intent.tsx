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

const stripePromise = loadStripe(
  process.env.NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY ?? "",
);

interface PaymentIntentModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface EmbeddedPaymentFormProps {
  onClose: () => void;
}

const EmbeddedPaymentForm = ({ onClose }: EmbeddedPaymentFormProps) => {
  const stripe = useStripe();
  const elements = useElements();
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();

    setIsLoading(true);

    if (!stripe || !elements) {
      setIsLoading(false);
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

    stripe.retrievePaymentIntent(clientSecret);
  }, [stripe]);

  return (
    <form onSubmit={handleSubmit}>
      <PaymentElement />
      <PrimaryButton
        className="rounded-sm mt-6 w-full bg-neutral-100 text-neutral-800 border-0 hover:bg-neutral-100 hover:text-neutral-800 hover:border-0"
        type="submit"
        loading={isLoading}
        disabled={!stripe || !elements}
      >
        Add Payment
      </PrimaryButton>
    </form>
  );
};

const PaymentIntentModal = ({ isOpen, onClose }: PaymentIntentModalProps) => {
  const [clientSecret, setClientSecret] = useState("");

  const appearance = {
    theme: "minimal",
  };
  const options = {
    clientSecret,
    appearance,
  };

  useEffect(() => {
    fetch("/api/stripe/paymentIntent", {
      method: "POST",
    })
      .then((res) => res.json())
      .then((data) => {
        setClientSecret(data.clientSecret);
      });
  }, []);

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
              <Dialog.Panel className="bg-white border-0 transform overflow-y-scroll scrollbar-hidden rounded-lg p-12 text-left align-middle transition-all">
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
