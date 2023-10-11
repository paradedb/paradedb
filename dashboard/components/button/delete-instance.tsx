"use client";

import { useState } from "react";
import { ExclamationCircleIcon } from "@heroicons/react/outline";

import { SecondaryButton } from "@/components/tremor";

const DeleteInstanceButton = ({
  onDeleteInstance,
  ...props
}: React.ComponentProps<typeof SecondaryButton> & {
  onDeleteInstance: () => void;
}) => {
  const [loading, setLoading] = useState(false);

  const deleteInstance = () => {
    fetch("/api/databases", { method: "DELETE" });
  };

  const onClick = () => {
    setLoading(true);
    onDeleteInstance();
    deleteInstance();
  };

  return (
    <SecondaryButton
      icon={ExclamationCircleIcon}
      size="xl"
      onClick={onClick}
      loading={loading}
      {...props}
    >
      Delete Instance
    </SecondaryButton>
  );
};

export { DeleteInstanceButton };
