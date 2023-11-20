"use client";

import { useState } from "react";

import { PrimaryButton } from "@/components/tremor";
import { PaymentIntentModal } from "@/components/modal";

interface AddPaymentButtonProps {
  onRefresh: () => void;
  amount: number;
}

const AddPaymentButton = ({ onRefresh, amount }: AddPaymentButtonProps) => {
  const [isStripeModalOpen, setIsStripeModalOpen] = useState(false);
  const buttonDisabled = amount <= 0;

  const onClick = () => {
    setIsStripeModalOpen(true);
  };

  const onStripeModalClose = () => {
    setIsStripeModalOpen(false);
    onRefresh();
  };

  return (
    <>
      <PaymentIntentModal
        isOpen={isStripeModalOpen}
        onClose={onStripeModalClose}
        amount={amount}
      />
      <PrimaryButton onClick={onClick} size="xs" disabled={buttonDisabled}>
        Add Payment
      </PrimaryButton>
    </>
  );
};

export { AddPaymentButton };
