import React from "react";
import { Button } from "@tremor/react";

const PRIMARY_BUTTON =
  "text-emerald-400 bg-emerald-400 border border-emerald-400 bg-opacity-20 rounded-none hover:bg-emerald-400 hover:bg-opacity-30 hover:border-emerald-400 duration-500 mt-6 rounded-none";

const PrimaryButton = ({
  children,
  ...props
}: React.ComponentProps<typeof Button>) => {
  return (
    <Button className={`${PRIMARY_BUTTON} ${props.className ?? ""}`} {...props}>
      {children}
    </Button>
  );
};

PrimaryButton.displayName = "PrimaryButton";

export { PrimaryButton };
