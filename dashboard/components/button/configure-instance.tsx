"use client";

import useSWR from "swr";
import { useState } from "react";

import { ConfigureInstanceModal } from "@/components/modal";
import { PrimaryButton } from "@/components/tremor";

const fetcher = (url: string) => fetch(url).then((res) => res.json());

interface ConfigureInstanceButtonProps {
  onConfigureInstance: () => void;
  onRefresh: () => void;
}

const ConfigureInstanceButton = ({
  onConfigureInstance,
  onRefresh,
  ...props
}: React.ComponentProps<typeof PrimaryButton> &
  ConfigureInstanceButtonProps) => {
  const { data: prices } = useSWR("/api/stripe/prices", fetcher);
  const { data: subscriptions } = useSWR("/api/stripe/subscription", fetcher);

  const defaultPlan = subscriptions?.data[0]?.plan?.id;

  const [modalIsOpen, setModalIsOpen] = useState(false);

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
        prices={prices?.data}
        onRefresh={onRefresh}
      />
      <PrimaryButton size="md" onClick={onClick} {...props}>
        Change Plan
      </PrimaryButton>
    </>
  );
};

export { ConfigureInstanceButton };
