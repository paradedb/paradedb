import classNames from "classnames";
import React from "react";
import { Button } from "@tremor/react";

const DEFAULT_BUTTON =
  "border bg-opacity-20 rounded-sm hover:bg-opacity-30 duration-500";
const PRIMARY_BUTTON = classNames(
  DEFAULT_BUTTON,
  "text-emerald-400 bg-emerald-400 border-emerald-400 hover:bg-emerald-400 hover:border-emerald-400",
);
const SECONDARY_BUTTON = classNames(
  DEFAULT_BUTTON,
  "text-neutral-400 bg-neutral-400 border-neutral-400 hover:bg-neutral-400 hover:border-neutral-400",
);

const PrimaryButton = ({
  children,
  ...props
}: React.ComponentProps<typeof Button>) => {
  return (
    <Button
      className={classNames(PRIMARY_BUTTON, props.className ?? "")}
      {...props}
    >
      {children}
    </Button>
  );
};

const SecondaryButton = ({
  children,
  ...props
}: React.ComponentProps<typeof Button>) => {
  return (
    <Button
      className={classNames(SECONDARY_BUTTON, props.className ?? "")}
      {...props}
    >
      {children}
    </Button>
  );
};

PrimaryButton.displayName = "PrimaryButton";
SecondaryButton.displayName = "SecondaryButton";

export { PrimaryButton, SecondaryButton };
