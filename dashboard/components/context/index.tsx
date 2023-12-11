import { createContext, useContext, useState } from "react";

enum NotificationType {
  SUCCESS,
  ERROR,
  LOADING,
}

interface Notification {
  title: string;
  description: string;
  type: NotificationType;
  icon: JSX.Element;
}

interface AppState {
  notification?: Notification;
  setNotification?: React.Dispatch<
    React.SetStateAction<Notification | undefined>
  >;
}

const defaultAppState: AppState = {
  notification: undefined,
  setNotification: undefined,
};

const AppStateContext = createContext<AppState>(defaultAppState);
const AppStateProvider = ({ children }: { children: React.ReactNode }) => {
  const [notification, setNotification] = useState<Notification>();

  return (
    <AppStateContext.Provider
      value={{
        notification,
        setNotification,
      }}
    >
      {children}
    </AppStateContext.Provider>
  );
};

const useAppState = () => useContext(AppStateContext);

export { type Notification, NotificationType, useAppState, AppStateProvider };
