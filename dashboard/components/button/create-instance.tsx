"use client";

import { useState } from "react";
import { PrimaryButton } from "@/components/tremor";

const CreateInstanceButton = ({
  onCreateInstance,
  ...props
}: React.ComponentProps<typeof PrimaryButton> & {
  onCreateInstance: () => void;
}) => {
  const [loading, setLoading] = useState(false);

  const createInstance = () => {
    fetch("/api/databases", { method: "POST" });
  };

  const onClick = () => {
    setLoading(true);
    createInstance();
    onCreateInstance();
  };

  return (
    <PrimaryButton size="xl" onClick={onClick} loading={loading} {...props}>
      Create Instance
    </PrimaryButton>
  );
};

export { CreateInstanceButton };
