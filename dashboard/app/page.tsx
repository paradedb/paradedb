"use client";

import useSWR, { mutate } from "swr";
import { useEffect, useRef } from "react";
import { withPageAuthRequired } from "@auth0/nextjs-auth0/client";
import { Title, Grid, Col, Text, Flex, List, ListItem } from "@tremor/react";
import {
  ServerIcon,
  UserIcon,
  KeyIcon,
  ArrowRightIcon,
  ServerStackIcon,
} from "@heroicons/react/24/outline";

import Error from "./error";
import { Card } from "@/components/tremor";
import {
  CopyToClipboardButton,
  CreateInstanceButton,
  DeleteInstanceButton,
} from "@/components/button";
import { Card as CardSkeleton } from "@/components/skeleton";
import { redirect } from "next/navigation";

const DATABASE_CREDENTIALS_URL = `/api/databases/credentials`;
const DATABASE_STATUS_URL = `/api/databases/status`;

const ERR_EXPIRED_ACCESS_TOKEN = "ERR_EXPIRED_ACCESS_TOKEN";

const POLLING_INTERVAL_MS = 2500;
const fetcher = (uri: string) => fetch(uri).then((res) => res.json());

enum DeployStatus {
  UNKNOWN = "unknown",
  PENDING = "pending",
  RUNNING = "running",
}

interface InstanceCardProps {
  creds: any;
  status: any;
  onCreateInstance: () => void;
  onDeleteInstance: () => void;
  databaseReady: boolean;
}

const CredentialsListItem = ({
  name,
  value,
  icon,
  hide,
}: {
  name: string;
  value: string;
  icon: JSX.Element;
  hide: boolean;
}) => (
  <ListItem>
    <Flex justifyContent="start" className="space-x-4">
      {icon}
      <Text className="text-neutral-400">{name}</Text>
    </Flex>
    <Flex justifyContent="end" className="overflow-none space-x-4">
      <Text className="text-neutral-100 font-medium truncate max-w-xs">
        {hide ? (
          <span>
            &bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;&bull;
          </span>
        ) : (
          value
        )}
      </Text>
      <CopyToClipboardButton text={value} />
    </Flex>
  </ListItem>
);

const InstanceCard = ({
  creds,
  status,
  onCreateInstance,
  onDeleteInstance,
  databaseReady,
}: InstanceCardProps) => {
  const deployStatus = status?.deploy_status;
  const isCreating = deployStatus === DeployStatus.PENDING;

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

  if (databaseReady) {
    return (
      <Card>
        <Flex flexDirection="col" alignItems="start" className="space-y-6">
          <Title className="text-neutral-100">My Instance</Title>
          <List className="divide-none space-y-2">
            <CredentialsListItem
              name="Password"
              value={creds?.password}
              icon={<KeyIcon className="w-4 text-amber-400" />}
              hide={true}
            />
            <CredentialsListItem
              name="Host"
              value={creds?.host}
              icon={<ServerIcon className="w-4 text-blue-400" />}
              hide={false}
            />
            <CredentialsListItem
              name="Database"
              value={creds?.dbname}
              icon={<ServerStackIcon className="w-4 text-indigo-400" />}
              hide={false}
            />
            <CredentialsListItem
              name="User"
              value={creds?.user}
              icon={<UserIcon className="w-4 text-pink-400" />}
              hide={false}
            />
            <CredentialsListItem
              name="Port"
              value={creds?.port}
              icon={<ArrowRightIcon className="w-4 text-emerald-400" />}
              hide={false}
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
  const { data: creds } = useSWR(DATABASE_CREDENTIALS_URL, fetcher);
  const { data: status } = useSWR(DATABASE_STATUS_URL, fetcher);

  const deployStatus = status?.deploy_status;

  const credsRef = useRef(creds);
  const statusRef = useRef(deployStatus);

  const databaseReady =
    [creds?.host, creds?.user, creds?.password, creds?.port].every(Boolean) &&
    deployStatus === DeployStatus.RUNNING;

  useEffect(() => {
    credsRef.current = creds;
    statusRef.current = deployStatus;
  }, [creds, deployStatus]);

  const formatPsql = () => {
    if (!databaseReady) return "";
    return `psql -h ${creds?.host} -p ${creds?.port} -U ${creds?.user} -d ${creds?.dbname} -W`;
  };

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

  return (
    <Grid numItemsLg={10} className="gap-8 h-full">
      <Col numColSpanLg={6} className="h-full">
        <InstanceCard
          creds={creds}
          status={status}
          onCreateInstance={onCreateInstance}
          onDeleteInstance={onDeleteInstance}
          databaseReady={databaseReady}
        />
      </Col>
      {databaseReady && (
        <Col numColSpanLg={4} className="h-full space-y-8">
          <Card>
            <Flex>
              <Title className="text-neutral-100">
                Connect with <code>psql</code>
              </Title>
              <CopyToClipboardButton text={formatPsql()} />
            </Flex>
            <div className="mt-6">
              <code className="text-emerald-400 text-sm">{formatPsql()}</code>
            </div>
          </Card>
        </Col>
      )}
    </Grid>
  );
};

export default withPageAuthRequired(Index);
