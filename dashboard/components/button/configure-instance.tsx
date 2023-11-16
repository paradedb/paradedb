"use client";

import classNames from "classnames";
import { useState, useEffect, Fragment } from "react";
import { Dialog, Transition } from "@headlessui/react";
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
import { loadStripe } from "@stripe/stripe-js";
import {
  EmbeddedCheckoutProvider,
  EmbeddedCheckout,
} from "@stripe/react-stripe-js";

import { PrimaryButton, Card } from "@/components/tremor";
import { XMarkIcon } from "@heroicons/react/24/outline";
import { CheckIcon } from "@heroicons/react/24/solid";

const stripePromise = loadStripe(
  process.env.NEXT_PUBLIC_STRIPE_PUBLISHABLE_KEY ?? "",
);

interface ConfigureInstanceModalProps {
  isOpen: boolean;
  onClose: () => void;
  defaultPlan: string;
  showStripeModal: () => void;
  prices: any[] | undefined;
}

interface StripeModalProps {
  isOpen: boolean;
  onClose: () => void;
}

const StripeModal = ({ isOpen, onClose }: StripeModalProps) => {
  const [clientSecret, setClientSecret] = useState("");

  useEffect(() => {
    fetch("/api/stripe/checkout", {
      method: "POST",
    })
      .then((res) => res.json())
      .then((data) => {
        setClientSecret(data.clientSecret);
      });
  }, []);

  return (
    <Transition appear show={isOpen} as={Fragment}>
      <Dialog as="div" className="relative z-20" onClose={() => {}}>
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
              <Dialog.Panel className="w-full h-[calc(100vh-40px)] transform overflow-y-scroll scrollbar-hidden rounded-lg bg-darker border border-neutral-800 p-12 text-left align-middle transition-all">
                <Button
                  icon={XMarkIcon}
                  variant="light"
                  color={"white" as any}
                  size="xl"
                  onClick={onClose}
                  className="fixed top-12 right-12"
                />
                <div className="mt-4">
                  {clientSecret && (
                    <EmbeddedCheckoutProvider
                      stripe={stripePromise}
                      options={{ clientSecret }}
                    >
                      <EmbeddedCheckout />
                    </EmbeddedCheckoutProvider>
                  )}
                </div>
              </Dialog.Panel>
            </Transition.Child>
          </div>
        </div>
      </Dialog>
    </Transition>
  );
};

const ConfigureInstanceModal = ({
  isOpen,
  onClose,
  defaultPlan,
  showStripeModal,
  prices,
}: ConfigureInstanceModalProps) => {
  const [selectedPlan, setSelectedPlan] = useState(defaultPlan);

  return (
    <>
      <Transition appear show={isOpen} as={Fragment}>
        <Dialog as="div" className="relative z-10" onClose={() => {}}>
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
                <Dialog.Panel className="w-full h-[calc(100vh-40px)] transform overflow-y-scroll scrollbar-hidden rounded-lg bg-darker border border-neutral-800 p-12 text-left align-middle transition-all">
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
                                  {price.unit_amount / 100}/mo
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
                    <PrimaryButton onClick={showStripeModal} size="xs">
                      Add Payment
                    </PrimaryButton>
                  </div>
                  <hr className="w-full border-neutral-800 mt-12" />
                  <Flex className="justify-start space-x-6 mt-12">
                    <Button
                      size="md"
                      className="bg-emerald-400 bg-opacity-20 text-emerald-400 border-0 hover:bg-emerald-400 hover:text-emerald-400 hover:border-emerald-400 hover:bg-opacity-30 duration-100"
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
                </Dialog.Panel>
              </Transition.Child>
            </div>
          </div>
        </Dialog>
      </Transition>
    </>
  );
};

const ConfigureInstanceButton = ({
  onConfigureInstance,
  ...props
}: React.ComponentProps<typeof PrimaryButton> & {
  onConfigureInstance: () => void;
}) => {
  const [modalIsOpen, setModalIsOpen] = useState(false);
  const [showStripeModal, setShowStripeModal] = useState(false);
  const [prices, setPrices] = useState<any[]>();

  useEffect(() => {
    fetch("/api/stripe/prices", {
      method: "GET",
    })
      .then((res) => res.json())
      .then((data) => setPrices(data?.prices));
  }, []);

  const onClick = () => {
    setModalIsOpen(true);
    onConfigureInstance();
  };

  const onCloseModal = () => {
    setModalIsOpen(false);
  };

  return (
    <>
      <ConfigureInstanceModal
        isOpen={modalIsOpen}
        onClose={onCloseModal}
        defaultPlan={""}
        showStripeModal={() => {
          setShowStripeModal(true);
        }}
        prices={prices}
      />
      <StripeModal
        isOpen={showStripeModal}
        onClose={() => setShowStripeModal(false)}
      />
      <PrimaryButton size="md" onClick={onClick} {...props}>
        Change Plan
      </PrimaryButton>
    </>
  );
};

export { ConfigureInstanceButton };
