"use client";

import { useState } from "react";
import { PlusIcon } from "@heroicons/react/24/outline";
import { SuccessButton } from "@/components/tremor";

const CreateInstanceButton = ({
  onCreateInstance,
  isCreating,
  ...props
}: React.ComponentProps<typeof SuccessButton> & {
  onCreateInstance: () => void;
  isCreating: boolean;
}) => {
  const [loading, setLoading] = useState(isCreating);

  const createInstance = () => {
    // TODO: don't hardcode id?
    fetch("/api/databases", {
      method: "POST",
      body: JSON.stringify({
        plan: "price_1ODBnKFLdqcXYNJa8VWQPkw8",
      }),
    });
  };

  const onClick = () => {
    setLoading(true);
    createInstance();
    onCreateInstance();
  };

  return (
    <SuccessButton
      size="md"
      icon={PlusIcon}
      onClick={onClick}
      loading={loading}
      {...props}
    >
      Create Instance
    </SuccessButton>
  );
};

export { CreateInstanceButton };
