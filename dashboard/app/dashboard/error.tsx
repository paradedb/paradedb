"use client";

import { useEffect } from "react";
import { Title, Text, Icon, Flex, Card } from "@tremor/react";
import { ExclamationCircleIcon } from "@heroicons/react/outline";

export default function Error({
  error,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    /* TODO: Log error to reporting service */
    console.error(error);
  }, [error]);

  return (
    <Card className="shadow-none">
      <Flex flexDirection="col">
        <Icon
          icon={ExclamationCircleIcon}
          size="lg"
          color="red"
          variant="light"
        />
        <Title className="mt-4">An unexpected error occured</Title>
        <Text className="mt-2">
          We&apos;re extremely sorry. If the error persists after reloading,
          please contact support@paradedb.com.
        </Text>
      </Flex>
    </Card>
  );
}
