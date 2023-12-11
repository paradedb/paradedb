import classNames from "classnames";
import React from "react";
import { Button } from "@tremor/react";

const DEFAULT_BUTTON = "rounded-sm duration-500";
const BUTTON_TYPES = {
  success:
    "text-emerald-400 bg-emerald-400 border-emerald-400 hover:bg-emerald-400 hover:border-emerald-400 bg-opacity-20 hover:bg-opacity-30",
  warning:
    "text-red-400 border-0 bg-red-400 bg-opacity-20 hover:bg-opacity-30 hover:text-red-400 hover:bg-red-400 bg-opacity-20 hover:bg-opacity-30",
  primary:
    "bg-neutral-100 text-neutral-800 border-0 hover:bg-neutral-100 hover:text-neutral-800 hover:border-0",
};

const BaseButton = ({
  children,
  buttonType,
  ...props
}: React.ComponentProps<typeof Button> & {
  buttonType: keyof typeof BUTTON_TYPES;
}) => {
  return (
    <Button
      className={classNames(DEFAULT_BUTTON, BUTTON_TYPES[buttonType])}
      {...props}
    >
      {children}
    </Button>
  );
};

const PrimaryButton = ({
  children,
  ...props
}: React.ComponentProps<typeof Button>) =>
  BaseButton({ children, buttonType: "primary", ...props });

const WarningButton = ({
  children,
  ...props
}: React.ComponentProps<typeof Button>) =>
  BaseButton({ children, buttonType: "warning", ...props });

const SuccessButton = ({
  children,
  ...props
}: React.ComponentProps<typeof Button>) =>
  BaseButton({ children, buttonType: "success", ...props });

PrimaryButton.displayName = "PrimaryButton";
WarningButton.displayName = "WarningButton";
SuccessButton.displayName = "SuccessButton";

export { PrimaryButton, WarningButton, SuccessButton };
