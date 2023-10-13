"use client";

import Link from "next/link";
import useSWR, { mutate } from "swr";
import { useEffect, useRef } from "react";
import { withPageAuthRequired } from "@auth0/nextjs-auth0/client";
import { Title, Grid, Col, Text, Flex, List, ListItem } from "@tremor/react";
import {
  ServerIcon,
  UserIcon,
  KeyIcon,
  ArrowRightIcon,
  LightningBoltIcon,
  DatabaseIcon,
  DownloadIcon,
  EyeIcon,
} from "@heroicons/react/outline";

import Error from "./error";
import { Card } from "@/components/tremor";
import {
  CreateInstanceButton,
  DeleteInstanceButton,
} from "@/components/button";
import { Card as CardSkeleton } from "@/components/skeleton";
import { redirect } from "next/navigation";

const DATABASE_CREDENTIALS_URL = `/api/databases/credentials`;
const DATABASE_STATUS_URL = `/api/databases/status`;
const IMPORTING_DATA_URL = "https://docs.paradedb.com/import";
const QUICKSTART_URL = "https://docs.paradedb.com/quickstart";
const SEARCH_BASICS_URL = "https://docs.paradedb.com/search/bm25";

const ERR_EXPIRED_ACCESS_TOKEN = "ERR_EXPIRED_ACCESS_TOKEN";

enum DeployStatus {
  UNKNOWN = "unknown",
  PENDING = "pending",
  RUNNING = "running",
}

const POLLING_INTERVAL_MS = 2500;

const fetcher = (uri: string) => fetch(uri).then((res) => res.json());

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

const InstanceCard = () => {
  const { data: creds } = useSWR(DATABASE_CREDENTIALS_URL, fetcher);
  const { data: status } = useSWR(DATABASE_STATUS_URL, fetcher);

  const deployStatus = status?.deploy_status;
  const isCreating = deployStatus === DeployStatus.PENDING;

  const credsRef = useRef(creds);
  const statusRef = useRef(deployStatus);

  useEffect(() => {
    credsRef.current = creds;
    statusRef.current = deployStatus;
  }, [creds, deployStatus]);

  const onCreateInstance = async () => {
    while (true) {
      const localDeployStatus = statusRef.current;

      if (localDeployStatus === DeployStatus.RUNNING) break;

      await mutate(DATABASE_STATUS_URL);
      await new Promise((resolve) => setTimeout(resolve, POLLING_INTERVAL_MS));
    }

    await mutate(DATABASE_CREDENTIALS_URL);
  };

  const onDeleteInstance = async () => {
    while (true) {
      const localDeployStatus = statusRef.current;

      if (localDeployStatus !== DeployStatus.RUNNING) break;

      await mutate(DATABASE_STATUS_URL);
      await new Promise((resolve) => setTimeout(resolve, POLLING_INTERVAL_MS));
    }

    await mutate(DATABASE_CREDENTIALS_URL);
  };

  if (creds?.status === 500 && creds?.message === ERR_EXPIRED_ACCESS_TOKEN) {
    redirect("/api/auth/logout");
  }

  if (creds?.status === 500) {
    return <Error />;
  }

  if (!creds || !status) {
    return (
      <Card>
        <Title className="text-neutral-100">My Instance</Title>
        <hr className="border-neutral-700 h-1 w-full my-6" />
        <CardSkeleton />
      </Card>
    );
  }

  if (
    [creds?.host, creds?.user, creds?.password, creds?.port].every(Boolean) &&
    deployStatus === DeployStatus.RUNNING
  ) {
    return (
      <Card>
        <Flex flexDirection="col" alignItems="start" className="space-y-6">
          <Title className="text-neutral-100">My Instance</Title>
          <hr className="border-neutral-700 h-1 w-full" />
          <List className="divide-none space-y-2">
            <CredentialsListItem
              name="Host"
              value={creds?.host}
              icon={<ServerIcon className="w-4 text-blue-400" />}
            />
            <CredentialsListItem
              name="Database"
              value={creds?.dbname}
              icon={<DatabaseIcon className="w-4 text-indigo-400" />}
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
              icon={<ArrowRightIcon className="w-4 text-emerald-400" />}
            />
          </List>
          <DeleteInstanceButton onDeleteInstance={onDeleteInstance} />
        </Flex>
      </Card>
    );
  }

  return (
    <Card>
      <Flex flexDirection="col" alignItems="start" className="space-y-6">
        <Title className="text-neutral-100">My Instance</Title>
        <hr className="border-neutral-700 h-1 w-full" />
        {isCreating ? (
          <Text className="mt-2 text-neutral-300">
            Your instance is creating. This could take a few minutes...
          </Text>
        ) : (
          <Text className="mt-2 text-neutral-300">
            You have not created a database instance.
          </Text>
        )}
        <CreateInstanceButton
          onCreateInstance={onCreateInstance}
          isCreating={isCreating}
        />
      </Flex>
    </Card>
  );
};

const Index = () => {
  return (
    <Grid numItemsLg={5} className="gap-8 h-full">
      <Col numColSpanLg={3} className="h-full">
        <InstanceCard />
      </Col>
      <Col numColSpanLg={2} className="h-full space-y-8">
        <Card>
          <Flex flexDirection="col" alignItems="start" className="space-y-6">
            <Title className="text-neutral-100">Guides</Title>
            <hr className="border-neutral-700 h-1 w-full" />
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
          </Flex>
        </Card>
      </Col>
    </Grid>
  );
};

export default withPageAuthRequired(Index);
