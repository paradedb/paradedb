import { Fragment } from "react";
import { Transition, Popover } from "@headlessui/react";
import { XMarkIcon } from "@heroicons/react/20/solid";
import { Button, Text } from "@tremor/react";

import { NotificationType, useAppState } from "@/components/context";
import { CheckCircleIcon } from "@heroicons/react/24/outline";
import { ExclamationCircleIcon } from "@heroicons/react/24/solid";

const GENERIC_LOADING = {
  title: "Saving changes",
  description: `Please allow a few seconds while your changes are being processed`,
  type: NotificationType.LOADING,
  icon: (
    <Button
      loading={true}
      variant="light"
      size="lg"
      className="text-neutral-300"
    />
  ),
};

const GENERIC_SUCCESS = {
  title: "Changes saved",
  description: `Your changes have been successfully saved`,
  type: NotificationType.SUCCESS,
  icon: <CheckCircleIcon className="text-emerald-500 w-6 h-6" />,
};

const GENERIC_ERROR = {
  title: "An error occurred",
  description: `Please try again later`,
  type: NotificationType.ERROR,
  icon: <ExclamationCircleIcon className="text-red-500 w-6 h-6" />,
};

const NotificationModal = () => {
  const { notification, setNotification } = useAppState();
  const isOpen = notification !== undefined;

  const hideNotification = () => setNotification?.(undefined);

  return (
    <Popover className="z-50 fixed top-8 right-8">
      <Transition appear show={isOpen} as={Fragment}>
        <Popover.Panel className="z-50 w-96 transform overflow-y-scroll scrollbar-hidden rounded-lg bg-neutral-900 border border-neutral-800 p-4 text-left transition-all">
          <Button
            icon={XMarkIcon}
            variant="light"
            size="lg"
            onClick={hideNotification}
            className="fixed top-4 right-4 text-gray-300 hover:text-gray-200"
          />
          <div className="flex items-start space-x-4">
            <div className="flex-shrink-0">{notification?.icon}</div>
            <div className="ml-3 w-0 flex-1 pt-0.5">
              <Text className="text-gray-100 font-semibold">
                {notification?.title}
              </Text>
              <Text className="text-gray-300 mt-1">
                {notification?.description}
              </Text>
            </div>
          </div>
        </Popover.Panel>
      </Transition>
    </Popover>
  );
};

export { NotificationModal, GENERIC_LOADING, GENERIC_SUCCESS, GENERIC_ERROR };
