"use client";

import classNames from "classnames";
import useSWR, { mutate } from "swr";
import { useState, Fragment } from "react";
import { Popover, Transition } from "@headlessui/react";
import {
  Flex,
  Bold,
  Text,
  Title,
  Metric,
  Table,
  TableHead,
  TableHeaderCell,
  TableBody,
  TableRow,
  TableCell,
  Button,
  Icon,
} from "@tremor/react";
import { CreditCardIcon, XMarkIcon } from "@heroicons/react/24/outline";
import { CheckIcon } from "@heroicons/react/24/solid";

import { Card } from "@/components/tremor";
import { AddPaymentButton } from "@/components/button/add-payment";
import { useAppState } from "../context";
import { GENERIC_LOADING, GENERIC_SUCCESS } from "@/components/modal";

interface ConfigureInstanceModalProps {
  isOpen: boolean;
  defaultPlan: string;
  prices: any[] | undefined;
  onClose: () => void;
  onRefresh: () => void;
}

const fetcher = (uri: string) => fetch(uri).then((res) => res.json());
const PAYMENT_METHODS_URL = `/api/stripe/paymentMethod`;
const SUBSCRIPTION_URL = `/api/stripe/subscription`;

const capitalize = (str: string) => str.charAt(0).toUpperCase() + str.slice(1);

const ConfigureInstanceModal = ({
  isOpen,
  defaultPlan,
  prices,
  onClose,
  onRefresh,
}: ConfigureInstanceModalProps) => {
  const [selectedPlan, setSelectedPlan] = useState(defaultPlan);
  const [isRemovingPayment, setIsRemovingPayment] = useState(false);
  const [isFinishing, setIsFinishing] = useState(false);
  const { setNotification } = useAppState();

  const { data: paymentMethods } = useSWR(PAYMENT_METHODS_URL, fetcher);
  const { data: subscriptions } = useSWR(SUBSCRIPTION_URL, fetcher);
  const paymentMethod = paymentMethods?.data?.[0];
  const subscriptionId = subscriptions?.data?.[0]?.id;
  const selectedAmount = prices?.find((price) => price.id === selectedPlan)
    ?.unit_amount;
  const canFinish =
    (selectedAmount > 0 && paymentMethod) || selectedAmount === 0;

  const onRefreshPaymentMethods = () => {
    mutate(PAYMENT_METHODS_URL);
  };

  const onClickFinish = async () => {
    setIsFinishing(true);
    setNotification?.(GENERIC_LOADING);

    const method = subscriptions?.data?.length > 0 ? "PUT" : "POST";
    await fetch("/api/stripe/subscription", {
      method,
      body: JSON.stringify({
        priceId: selectedPlan,
        subscriptionId,
      }),
    });

    // @mauricio
    // TODO: Send API call to backend to swap out instance
    // You can get the selected plan and instance specs via
    // prices?.find((price) => price.id === selectedPlan)

    onClose();
    onRefresh();
    setIsFinishing(false);
    setNotification?.(GENERIC_SUCCESS);
  };

  const removePaymentMethod = async () => {
    setIsRemovingPayment(true);

    await fetch(PAYMENT_METHODS_URL, {
      method: "DELETE",
      body: JSON.stringify({
        paymentMethodId: paymentMethod.id,
      }),
    });

    onRefreshPaymentMethods();
    setIsRemovingPayment(false);
  };

  return (
    <>
      <Transition appear show={isOpen} as={Fragment}>
        <Popover className="z-40">
          <Transition.Child
            as={Fragment}
            enter="ease-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in duration-200"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <div className="fixed inset-0 bg-black/25" />
          </Transition.Child>

          <div className="fixed inset-0 overflow-y-auto">
            <div className="flex min-h-full items-center justify-center p-4 text-center">
              <Transition.Child
                as={Fragment}
                enter="ease-out duration-300"
                enterFrom="opacity-0 scale-95"
                enterTo="opacity-100 scale-100"
                leave="ease-in duration-200"
                leaveFrom="opacity-100 scale-100"
                leaveTo="opacity-0 scale-95"
              >
                <Popover.Panel className="w-full h-[calc(100vh-30px)] transform overflow-y-scroll scrollbar-hidden rounded-lg bg-darker border border-neutral-800 p-12 text-left align-middle transition-all">
                  <Button
                    icon={XMarkIcon}
                    variant="light"
                    color={"white" as any}
                    size="xl"
                    onClick={onClose}
                    className="fixed top-12 right-12"
                  />
                  <Metric className="text-gray-100">Change Plan</Metric>
                  <Title className="mt-8 text-gray-100">Plan Type</Title>
                  <Text className="text-gray-300 mt-4">
                    Every ParadeDB instance runs inside a dedicated virtual
                    machine. You can configure the memory and compute of your
                    instance by selecting one of the plans below.
                  </Text>
                  <Card className="mt-6 bg-dark ring-neutral-800">
                    <Table className="mt-5">
                      <TableHead>
                        <TableRow>
                          <TableHeaderCell></TableHeaderCell>
                          <TableHeaderCell className="text-gray-100">
                            Plan
                          </TableHeaderCell>
                          <TableHeaderCell className="text-gray-100">
                            Price
                          </TableHeaderCell>
                          <TableHeaderCell className="text-gray-100">
                            CPU Cores
                          </TableHeaderCell>
                          <TableHeaderCell className="text-gray-100">
                            Memory
                          </TableHeaderCell>
                          <TableHeaderCell className="text-gray-100">
                            Storage
                          </TableHeaderCell>
                          <TableHeaderCell className="text-gray-100">
                            High Availability
                          </TableHeaderCell>
                        </TableRow>
                      </TableHead>
                      <TableBody className="divide-neutral-800">
                        {prices?.map((price) => {
                          const isSelected = selectedPlan === price.id;
                          const textClass = isSelected
                            ? "text-gray-900 font-medium"
                            : "text-gray-300";

                          return (
                            <TableRow
                              key={price.id}
                              className={classNames(
                                "cursor-pointer duration-100 rounded",
                                isSelected && "bg-neutral-100",
                              )}
                              onClick={() => setSelectedPlan(price.id)}
                            >
                              <TableCell className="max-w-[8px]">
                                {isSelected ? (
                                  <Icon
                                    icon={CheckIcon}
                                    variant="simple"
                                    size="sm"
                                    className={textClass}
                                  ></Icon>
                                ) : (
                                  <div className="py-4"></div>
                                )}
                              </TableCell>
                              <TableCell className={textClass}>
                                {price.nickname}
                              </TableCell>
                              <TableCell>
                                <Text className={textClass}>
                                  ${price.unit_amount / 100}/mo
                                </Text>
                              </TableCell>
                              <TableCell>
                                <Text className={textClass}>
                                  {price.metadata.cpu}
                                </Text>
                              </TableCell>
                              <TableCell>
                                <Text className={textClass}>
                                  {price.metadata.memory}GB
                                </Text>
                              </TableCell>
                              <TableCell>
                                <Text className={textClass}>
                                  {price.metadata.storage}GB
                                </Text>
                              </TableCell>
                              <TableCell>
                                <Text className={textClass}>
                                  {price.metadata.highAvailability}
                                </Text>
                              </TableCell>
                            </TableRow>
                          );
                        })}
                      </TableBody>
                    </Table>
                  </Card>
                  <Title className="mt-8 text-gray-100">Selected Plan</Title>
                  <Text className="text-gray-300 mt-4">
                    You have selected the{" "}
                    <Bold className="text-emerald-400">
                      {
                        prices?.find((price) => price.id === selectedPlan)
                          ?.nickname
                      }
                    </Bold>
                    . This plan will take effect immediately.
                  </Text>
                  <Title className="mt-8 text-gray-100">Payment Method</Title>
                  <div className="mt-4">
                    {paymentMethod ? (
                      <>
                        <Flex className="space-x-4 justify-start">
                          <Icon
                            icon={CreditCardIcon}
                            variant="simple"
                            color="emerald"
                            size="md"
                          />
                          <div>
                            <Text className="text-gray-100">
                              <Bold>
                                {capitalize(paymentMethod.card?.brand)} ending
                                in {paymentMethod.card?.last4}
                              </Bold>
                            </Text>
                            <Text className="text-gray-300 mt-1">
                              Expiry in {paymentMethod.card?.exp_month}/
                              {paymentMethod.card?.exp_year}
                            </Text>
                          </div>
                        </Flex>
                        <Button
                          loading={isRemovingPayment}
                          variant="light"
                          className="text-red-400 mt-6 hover:text-red-500 duration-100"
                          onClick={removePaymentMethod}
                        >
                          Remove Payment Method
                        </Button>
                      </>
                    ) : (
                      <AddPaymentButton
                        onRefresh={onRefreshPaymentMethods}
                        amount={selectedAmount}
                      />
                    )}
                  </div>
                  <hr className="w-full border-neutral-800 mt-12" />
                  <Flex className="justify-start space-x-6 mt-12">
                    <Button
                      disabled={!canFinish}
                      loading={isFinishing}
                      size="md"
                      className="bg-emerald-400 bg-opacity-20 text-emerald-400 border-0 hover:bg-emerald-400 hover:text-emerald-400 hover:border-emerald-400 hover:bg-opacity-30 duration-100"
                      onClick={onClickFinish}
                    >
                      Finish & Confirm
                    </Button>
                    <Button
                      onClick={onClose}
                      size="md"
                      variant="light"
                      className="text-gray-300 hover:text-gray-300"
                    >
                      Go Back
                    </Button>
                  </Flex>
                </Popover.Panel>
              </Transition.Child>
            </div>
          </div>
        </Popover>
      </Transition>
    </>
  );
};

export { ConfigureInstanceModal };
