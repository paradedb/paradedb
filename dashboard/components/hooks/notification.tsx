import { Notification, useAppState } from "@/components/context";

const useNotification = () => {
  const { setNotification } = useAppState();

  const withNotification = (
    tasks: Array<Promise<Response | Array<Response>>>,
    before?: Notification,
    after?: Notification,
  ) => {
    if (!setNotification) return;

    setNotification(before);
    return Promise.all(tasks).then(() => {
      setNotification(after);
    });
  };

  return { withNotification };
};

export { useNotification };
