"use client";

import { useState, useEffect } from "react";

import { ConfigureInstanceModal, PaymentIntentModal } from "@/components/modal";
import { PrimaryButton } from "@/components/tremor";

const ConfigureInstanceButton = ({
  onConfigureInstance,
  ...props
}: React.ComponentProps<typeof PrimaryButton> & {
  onConfigureInstance: () => void;
}) => {
  const [modalIsOpen, setModalIsOpen] = useState(false);
  const [showPaymentIntentModal, setShowPaymentIntentModal] = useState(false);
  const [prices, setPrices] = useState<any[]>();

  useEffect(() => {
    fetch("/api/stripe/prices", {
      method: "GET",
    })
      .then((res) => res.json())
      .then((data) => setPrices(data?.prices));
  }, []);

  const onClick = () => {
    setModalIsOpen(true);
    onConfigureInstance();
  };

  const onCloseModal = () => {
    setModalIsOpen(false);
  };

  return (
    <>
      <ConfigureInstanceModal
        isOpen={modalIsOpen}
        onClose={onCloseModal}
        defaultPlan={""}
        showStripeModal={() => {
          setShowPaymentIntentModal(true);
        }}
        prices={prices}
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
