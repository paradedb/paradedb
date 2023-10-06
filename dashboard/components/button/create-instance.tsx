"use client";

import { useState } from "react";
import { PrimaryButton } from "@/components/tremor";

const CreateInstanceButton = ({
  ...props
}: React.ComponentProps<typeof PrimaryButton>) => {
  const [loading, setLoading] = useState(false);

  const createInstance = () => {
    fetch("/api/cloud", { method: "POST" });
  };

  const onClick = () => {
    setLoading(true);
    createInstance();
  };

  return (
    <PrimaryButton size="xl" onClick={onClick} loading={loading} {...props}>
      Create Instance
    </PrimaryButton>
  );
};

export { CreateInstanceButton };
