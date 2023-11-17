"use client";

import Image from "next/image";
import classname from "classnames";
import { useUser } from "@auth0/nextjs-auth0/client";
import { redirect } from "next/navigation";

import { IntercomProvider } from "react-use-intercom";
import { Grid, Col, Flex, Button, Text } from "@tremor/react";
import {
  CircleStackIcon,
  ArrowLeftIcon,
  BookOpenIcon,
} from "@heroicons/react/24/outline";
import { usePathname } from "next/navigation";

import { Spinner } from "@/components/skeleton";
import Logo from "@/images/logo-with-name.svg";

enum Route {
  Dashboard = "/",
  Welcome = "/welcome",
  Settings = "/settings",
  Logout = "/api/auth/logout",
  Login = "/api/auth/login",
  Documentation = "https://docs.paradedb.com",
}

const SidebarButton = ({
  active,
  name,
  href,
  target,
  icon,
}: {
  active: boolean;
  name: string;
  href: string;
  target?: string;
  icon: React.ElementType;
}) => {
  const SIDEBAR_BUTTON_DEFAULT =
    "py-1 duration-500 text-gray-500 hover:text-emerald-300";
  const SIDEBAR_BUTTON_ACTIVE = "text-emerald-400";

  return (
    <a
      target={target ?? "_self"}
      href={href}
      className={classname(
        SIDEBAR_BUTTON_DEFAULT,
        active && SIDEBAR_BUTTON_ACTIVE,
      )}
    >
      <Button
        onClick={() => {
          redirect(href);
        }}
        icon={icon}
        variant="light"
        className={classname(
          "duration-500",
          SIDEBAR_BUTTON_DEFAULT,
          active && SIDEBAR_BUTTON_ACTIVE,
        )}
      >
        {name}
      </Button>
    </a>
  );
};

const DashboardLayout = ({ children }: { children: React.ReactNode }) => {
  const pathname = usePathname();
  const { user, isLoading } = useUser();

  const titleMap: {
    [key: string]: string;
  } = {
    [Route.Dashboard]: "Dashboard",
    [Route.Welcome]: "Welcome Letter",
    [Route.Settings]: "Settings",
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-screen bg-dark">
        <Spinner />
      </div>
    );
  }

  if (!user) {
    redirect(Route.Login);
  }

  return (
    <div className="w-screen h-screen fixed overflow-y-auto overflow-x-hidden">
      <Grid numItemsLg={12} className="w-screen">
        <Col
          numColSpanLg={2}
          numColSpanMd={2}
          numColSpanSm={0}
          className="min-h-screen bg-dark border-r-[1px] border-neutral-800 min-w-[220px] relative"
        >
          <div className="fixed top-6 left-8">
            <Image
              src={Logo}
              width={35}
              height={35}
              alt="ParadeDB"
              className="right-1 relative"
            />
            <Flex
              className="mt-8 space-y-2"
              flexDirection="col"
              alignItems="start"
            >
              <SidebarButton
                active={pathname === Route.Dashboard}
                href={Route.Dashboard}
                name="Dashboard"
                icon={CircleStackIcon}
              />
              <SidebarButton
                active={pathname === Route.Welcome}
                href={Route.Welcome}
                name="Welcome Letter"
                icon={BookOpenIcon}
              />
              {/* TODO: Create settings page */}
              {/* <SidebarButton
              active={pathname === Route.Settings}
              href={Route.Settings}
              name="Settings"
              icon={CogIcon}
            /> */}
            </Flex>
          </div>
          <div className="fixed bottom-6 left-8">
            <Flex flexDirection="col" alignItems="start">
              <SidebarButton
                active={false}
                href={Route.Documentation}
                target="_blank"
                name="Documentation"
                icon={BookOpenIcon}
              />
              <SidebarButton
                active={false}
                href={Route.Logout}
                name="Log Out"
                icon={ArrowLeftIcon}
              />
            </Flex>
          </div>
        </Col>
        <Col
          numColSpanLg={10}
          numColSpanMd={10}
          numColSpanSm={12}
          className="pb-10 bg-dark w-full"
        >
          <Flex className="justify-start text-neutral-500 border-b border-neutral-800 py-4 px-8 fixed w-full z-20 bg-dark space-x-3">
            <Text>/</Text>
            <Text>{titleMap[pathname]}</Text>
          </Flex>
          <div className="mt-20 px-8">{children}</div>
        </Col>
      </Grid>
    </div>
  );
};

const Template = ({ children }: { children: React.ReactNode }) => (
  <IntercomProvider autoBoot appId={process.env.INTERCOM_APP_ID ?? ""}>
    <DashboardLayout>{children}</DashboardLayout>
  </IntercomProvider>
);

export default Template;
