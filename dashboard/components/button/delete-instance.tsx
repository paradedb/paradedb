"use client";

import { useState } from "react";
import { ExclamationCircleIcon } from "@heroicons/react/24/outline";

import { WarningButton } from "@/components/tremor";

const DeleteInstanceButton = ({
  onDeleteInstance,
  ...props
}: React.ComponentProps<typeof WarningButton> & {
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
    <WarningButton
      icon={ExclamationCircleIcon}
      size="md"
      onClick={onClick}
      loading={loading}
      {...props}
    >
      Delete Instance
    </WarningButton>
  );
};

export { DeleteInstanceButton };
