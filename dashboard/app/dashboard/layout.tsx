"use client";
import Image from "next/image";
import Link from "next/link";
import Logo from "../../images/logo-with-name.svg";
import classname from "classnames";

import { Grid, Col, Flex, Button, Metric } from "@tremor/react";
import {
  HomeIcon,
  CogIcon,
  ArrowNarrowLeftIcon,
} from "@heroicons/react/outline";
import { usePathname } from "next/navigation";

enum Route {
  Dashboard = "/dashboard",
  Settings = "/settings",
  Logout = "/api/auth/logout",
}

const SIDEBAR_BUTTON_DEFAULT = "w-full px-6 pb-2 pt-3 rounded-md";
const SIDEBAR_BUTTON_ACTIVE = "bg-emerald-300";

const SidebarButton = ({
  active,
  name,
  href,
  icon,
}: {
  active: boolean;
  name: string;
  href: string;
  icon: (props: React.ComponentProps<"svg">) => JSX.Element;
}) => {
  return (
    <Link
      href={href}
      className={classname(
        SIDEBAR_BUTTON_DEFAULT,
        active && SIDEBAR_BUTTON_ACTIVE,
      )}
    >
      <Button
        icon={icon}
        variant="light"
        color={active ? "black" : ("slate" as any)}
      >
        {name}
      </Button>
    </Link>
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
    <section className="fixed">
      <Grid numItemsLg={10} className="gap-6">
        <Col
          numColSpanLg={2}
          numColSpanMd={2}
          numColSpanSm={0}
          className="min-h-screen bg-stone-100 p-4 border-r-[1px] border-stone-800"
        >
          <Image src={Logo} width={125} height={50} alt="ParadeDB" />
          <Flex
            className="mt-12 space-y-2"
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
              active={pathname === Route.Settings}
              href={Route.Settings}
              name="Settings"
              icon={CogIcon}
            />
            <div className="absolute bottom-6 -left-[1px]">
              <SidebarButton
                active={false}
                href={Route.Logout}
                name="Log Out"
                icon={ArrowNarrowLeftIcon}
              />
            </div>
          </Flex>
        </Col>
        <Col
          numColSpanLg={8}
          numColSpanMd={8}
          numColSpanSm={10}
          className="px-4 py-8"
        >
          <Metric color="slate">{titleMap[pathname]}</Metric>
          <div className="mt-4">{children}</div>
        </Col>
      </Grid>
    </section>
  );
};

export default DashboardLayout;
