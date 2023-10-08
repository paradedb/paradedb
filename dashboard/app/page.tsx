"use client";

import Link from "next/link";
import useSWR, { mutate } from "swr";

import { useState } from "react";
import { withPageAuthRequired } from "@auth0/nextjs-auth0/client";
import {
  Title,
  Grid,
  Col,
  Text,
  Flex,
  List,
  ListItem,
  Divider,
} from "@tremor/react";
import {
  ServerIcon,
  UserIcon,
  KeyIcon,
  ArrowRightIcon,
  LightningBoltIcon,
  DownloadIcon,
  EyeIcon,
} from "@heroicons/react/outline";

import { Card } from "@/components/tremor";
import { CreateInstanceButton } from "@/components/button";

const DATABASE_CREDENTIALS_URL = `/api/databases/credentials`;
const DATABASE_STATUS_URL = `/api/databases/status`;
const IMPORTING_DATA_URL = "https://docs.paradedb.com/import";
const QUICKSTART_URL = "https://docs.paradedb.com/quickstart";
const SEARCH_BASICS_URL = "https://docs.paradedb.com/search/bm25";

enum DeployStatus {
  UNKNOWN = "unknown",
}

const fetcher = (uri: string) => fetch(uri).then((res) => res.json());

// const getDatabaseCredentials = async () =>
//   await fetch(DATABASE_CREDENTIALS_URL, {
//     method: "GET",
//   });

// const getDatabaseStatus = async () =>
//   await fetch(DATABASE_STATUS_URL, {
//     method: "GET",
//   });

const CredentialsListItem = ({
  name,
  value,
  icon,
}: {
  name: string;
  value: string;
  icon: JSX.Element;
}) => (
  <ListItem>
    <Flex justifyContent="start" className="space-x-4">
      {icon}
      <Text className="text-neutral-400">{name}</Text>
    </Flex>
    <Text className="text-neutral-100 font-medium">{value}</Text>
  </ListItem>
);

const GuideListItem = ({
  href,
  name,
  icon,
}: {
  href: string;
  name: string;
  icon: JSX.Element;
}) => (
  <Link href={href} target="_blank">
    <ListItem className="py-2.5">
      <Flex justifyContent="start" className="space-x-4">
        {icon}
        <Text className="text-neutral-100 font-medium">{name}</Text>
      </Flex>
      <ArrowRightIcon className="w-4 text-neutral-400" />
    </ListItem>
  </Link>
);

const Dashboard = () => {
  const { data: creds } = useSWR(DATABASE_CREDENTIALS_URL, fetcher);
  const { data: status } = useSWR(DATABASE_STATUS_URL, fetcher);

  return (
    <Grid numItemsLg={5} className="gap-10 h-full">
      <Col numColSpanLg={3} className="h-full">
        <Card>
          <Title className="text-neutral-100">My Instance</Title>
          <Divider className="bg-neutral-600" />
          {creds ? (
            <List className="divide-none space-y-2">
              <CredentialsListItem
                name="Host"
                value={creds?.host}
                icon={<ServerIcon className="w-4 text-blue-400" />}
              />
              <CredentialsListItem
                name="User"
                value={creds?.user}
                icon={<UserIcon className="w-4 text-pink-400" />}
              />
              <CredentialsListItem
                name="Password"
                value={creds?.password}
                icon={<KeyIcon className="w-4 text-amber-400" />}
              />
              <CredentialsListItem
                name="Port"
                value={creds?.port}
                icon={<ArrowRightIcon className="w-4 text-purple-400" />}
              />
            </List>
          ) : status === DeployStatus.UNKNOWN ? (
            <Flex flexDirection="col" alignItems="start" className="space-y-6">
              <Text className="mt-2 text-neutral-300">
                You have not created a database instance.
              </Text>
              <CreateInstanceButton />
            </Flex>
          ) : (
            <>Creating Instance</>
          )}
        </Card>
      </Col>
      <Col numColSpanLg={2} className="h-full">
        <Card>
          <Title className="text-neutral-100">Guides</Title>
          <Divider className="bg-neutral-600" />
          <List className="divide-neutral-700 space-y-2">
            <GuideListItem
              href={IMPORTING_DATA_URL}
              name="Importing Data"
              icon={<DownloadIcon className="w-4 text-indigo-400" />}
            />
            <GuideListItem
              href={QUICKSTART_URL}
              name="Quickstart"
              icon={<LightningBoltIcon className="w-4 text-yellow-400" />}
            />
            <GuideListItem
              href={SEARCH_BASICS_URL}
              name="Search Basics"
              icon={<EyeIcon className="w-4 text-emerald-400" />}
            />
          </List>
        </Card>
      </Col>
    </Grid>
  );
};

export default withPageAuthRequired(Dashboard);
