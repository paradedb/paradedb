"use client";

import useSWR from "swr";
import { useState } from "react";

import { ConfigureInstanceModal, PaymentIntentModal } from "@/components/modal";
import { PrimaryButton } from "@/components/tremor";

const fetcher = (url: string) => fetch(url).then((res) => res.json());

const ConfigureInstanceButton = ({
  onConfigureInstance,
  ...props
}: React.ComponentProps<typeof PrimaryButton> & {
  onConfigureInstance: () => void;
}) => {
  const { data: prices } = useSWR("/api/stripe/prices", fetcher);
  const { data: subscriptions } = useSWR("/api/stripe/subscription", fetcher);

  const defaultPlan = subscriptions?.subscriptions?.data[0]?.plan?.id;

  const [modalIsOpen, setModalIsOpen] = useState(false);
  const [showPaymentIntentModal, setShowPaymentIntentModal] = useState(false);

  const onClick = () => {
    setModalIsOpen(true);
    onConfigureInstance();
  };

  const onCloseModal = () => {
    setModalIsOpen(false);
  };

  if (!defaultPlan) return <></>;

  return (
    <>
      <ConfigureInstanceModal
        isOpen={modalIsOpen}
        onClose={onCloseModal}
        defaultPlan={defaultPlan}
        showStripeModal={() => {
          setShowPaymentIntentModal(true);
        }}
        prices={prices?.prices}
      />
      <PaymentIntentModal
        isOpen={showPaymentIntentModal}
        onClose={() => setShowPaymentIntentModal(false)}
      />
      <PrimaryButton size="md" onClick={onClick} {...props}>
        Change Plan
      </PrimaryButton>
    </>
  );
};

export { ConfigureInstanceButton };
