"use client";

import { useState } from "react";
import { SecondaryButton } from "@/components/tremor";

const DeleteInstanceButton = ({
  ...props
}: React.ComponentProps<typeof SecondaryButton>) => {
  const [loading, setLoading] = useState(false);

  const deleteInstance = () => {
    fetch("/api/databases", { method: "DELETE" });
  };

  const onClick = () => {
    setLoading(true);
    deleteInstance();
  };

  return (
    <SecondaryButton size="xl" onClick={onClick} loading={loading} {...props}>
      Delete Instance
    </SecondaryButton>
  );
};

export { DeleteInstanceButton };
