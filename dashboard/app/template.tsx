"use client";

import Image from "next/image";
import classname from "classnames";
import { useUser } from "@auth0/nextjs-auth0/client";
import { redirect } from "next/navigation";

import { IntercomProvider } from "react-use-intercom";
import { Grid, Col, Flex, Button, Metric } from "@tremor/react";
import {
  HomeIcon,
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
    "w-full px-6 pb-2 pt-3 rounded-sm duration-500";
  const SIDEBAR_BUTTON_ACTIVE = "bg-emerald-400 hover:bg-emerald-300";

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
        icon={icon}
        variant="light"
        className={classname(
          "duration-500",
          active
            ? "text-neutral-900 hover:text-neutral-900"
            : "text-neutral-500 hover:text-emerald-400",
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
    <div className="fixed">
      <Grid numItemsLg={10} className="w-screen">
        <Col
          numColSpanLg={2}
          numColSpanMd={2}
          numColSpanSm={0}
          className="min-h-screen bg-dark px-8 py-8 border-r-[1px] border-neutral-800 min-w-[220px]"
        >
          <Image src={Logo} width={125} height={50} alt="ParadeDB" />
          <Flex
            className="mt-8 space-y-2"
            flexDirection="col"
            alignItems="start"
          >
            <SidebarButton
              active={pathname === Route.Dashboard}
              href={Route.Dashboard}
              name="Dashboard"
              icon={HomeIcon}
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
            <div className="absolute bottom-6 left-4">
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
          </Flex>
        </Col>
        <Col
          numColSpanLg={8}
          numColSpanMd={8}
          numColSpanSm={10}
          className="py-6 bg-dark"
        >
          <Metric className="text-neutral-100 font-semibold px-12">
            {titleMap[pathname]}
          </Metric>
          <hr className="border-neutral-800 h-1 w-full my-6" />
          <div className="mt-8 px-12">{children}</div>
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
