"use client";

import { Title, Text, Icon, Flex } from "@tremor/react";
import { ExclamationCircleIcon } from "@heroicons/react/24/outline";
import { useIntercom } from "react-use-intercom";

import { Card } from "@/components/tremor/card";

const Error = ({}: {
  error?: Error & { digest?: string };
  reset?: () => void;
}) => {
  const { show } = useIntercom();

  return (
    <Card>
      <Flex flexDirection="col">
        <Icon
          icon={ExclamationCircleIcon}
          size="lg"
          color="red"
          variant="light"
        />
        <Title className="mt-4 text-neutral-100">
          An unexpected error occured
        </Title>
        <Text className="mt-2 text-neutral-400">
          We&apos;re extremely sorry. Please{" "}
          <span
            onClick={show}
            className="text-neutral-100 font-medium cursor-pointer underline"
          >
            notify our support team
          </span>{" "}
          amd we&apos;ll get this fixed.
        </Text>
      </Flex>
    </Card>
  );
};

export default Error;
