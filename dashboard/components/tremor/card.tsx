import React, { forwardRef } from "react";
import { Card as TremorCard } from "@tremor/react";

const DEFAULT_CARD =
  "shadow-none bg-darker ring-neutral-800 rounded-sm px-12 py-8";

const Card = forwardRef<
  HTMLDivElement,
  React.ComponentProps<typeof TremorCard>
>(({ children, ...props }, ref) => {
  return (
    <TremorCard
      ref={ref}
      className={`${DEFAULT_CARD} ${props.className ?? ""}`}
      {...props}
    >
      {children}
    </TremorCard>
  );
});

Card.displayName = "Card";

export { Card };
