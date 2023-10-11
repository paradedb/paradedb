"use client";

import { Card } from "@/components/tremor/card";
import { PrimaryButton } from "@/components/tremor";
import { BookmarkIcon } from "@heroicons/react/outline";
import { Flex, Title } from "@tremor/react";

const FounderLetterCard = () => (
  <Card>
    <Flex flexDirection="col" alignItems="start" className="space-y-6">
      <Title className="text-neutral-100">A Letter from the Founders</Title>
      <hr className="border-neutral-700 h-1 w-full" />
      <PrimaryButton icon={BookmarkIcon} size="xl">
        Open Letter
      </PrimaryButton>
    </Flex>
  </Card>
);

export { FounderLetterCard };
