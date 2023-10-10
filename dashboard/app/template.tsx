"use client";

import Image from "next/image";
import Logo from "../images/logo-with-name.svg";
import classname from "classnames";

import { IntercomProvider } from "react-use-intercom";
import { Grid, Col, Flex, Button, Metric, Divider } from "@tremor/react";
import {
  HomeIcon,
  ArrowNarrowLeftIcon,
  BookOpenIcon,
} from "@heroicons/react/outline";
import { usePathname } from "next/navigation";

enum Route {
  Dashboard = "/",
  Settings = "/settings",
  Logout = "/api/auth/logout",
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
  icon: (props: React.ComponentProps<"svg">) => JSX.Element;
}) => {
  const SIDEBAR_BUTTON_DEFAULT =
    "w-full px-6 pb-2 pt-3 rounded-sm duration-500";
  const SIDEBAR_BUTTON_ACTIVE = "bg-emerald-400 hover:bg-emerald-300";

  return (
    <a
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
  const titleMap: {
    [key: string]: string;
  } = {
    [Route.Dashboard]: "Dashboard",
    [Route.Settings]: "Settings",
  };

  return (
    <div className="fixed">
      <Grid numItemsLg={10} className="w-screen">
        <Col
          numColSpanLg={2}
          numColSpanMd={2}
          numColSpanSm={0}
          className="min-h-screen bg-black px-8 py-8 border-r-[1px] border-neutral-800 min-w-[220px]"
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
                  name="Documentation"
                  target="_blank"
                  icon={BookOpenIcon}
                />
                <SidebarButton
                  active={false}
                  href={Route.Logout}
                  name="Log Out"
                  icon={ArrowNarrowLeftIcon}
                />
              </Flex>
            </div>
          </Flex>
        </Col>
        <Col
          numColSpanLg={8}
          numColSpanMd={8}
          numColSpanSm={10}
          className="px-12 py-6 bg-black overflow-y-scroll"
        >
          <Metric className="text-neutral-100">{titleMap[pathname]}</Metric>
          <Divider className="bg-neutral-800" />
          <div className="mt-8">{children}</div>
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
