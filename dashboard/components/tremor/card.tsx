import React, { forwardRef } from "react";
import { Card } from "@tremor/react";

const DARK_CARD =
  "shadow-none bg-neutral-900 ring-neutral-700 rounded-sm px-12 py-8";

const DarkCard = forwardRef<HTMLDivElement, React.ComponentProps<typeof Card>>(
  ({ children, ...props }, ref) => {
    return (
      <Card
        ref={ref}
        className={`${DARK_CARD} ${props.className ?? ""}`}
        {...props}
      >
        {children}
      </Card>
    );
  },
);

DarkCard.displayName = "DarkCard";

export { DarkCard };
