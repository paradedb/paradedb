"use client";

import useSWR, { mutate } from "swr";
import { useEffect, useRef } from "react";
import { withPageAuthRequired } from "@auth0/nextjs-auth0/client";
import {
  Title,
  Metric,
  Grid,
  Col,
  Text,
  Flex,
  List,
  ListItem,
  Bold,
  Icon,
} from "@tremor/react";
import {
  ServerIcon,
  UserIcon,
  KeyIcon,
  ArrowRightIcon,
  ServerStackIcon,
  CpuChipIcon,
  Square3Stack3DIcon,
} from "@heroicons/react/24/outline";

import Error from "./error";
import { Card } from "@/components/tremor";
import {
  ConfigureInstanceButton,
  CopyToClipboardButton,
  CreateInstanceButton,
  DeleteInstanceButton,
} from "@/components/button";
import { redirect } from "next/navigation";

const DATABASE_CREDENTIALS_URL = `/api/databases/credentials`;
const DATABASE_STATUS_URL = `/api/databases/status`;
// const USER_URL = `/api/auth/user`;

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
  isLoading: boolean;
}

const CredentialsListItem = ({
  name,
  value,
  icon,
  hide,
  loading,
}: {
  name: string;
  value: string;
  icon: JSX.Element;
  hide: boolean;
  loading: boolean;
}) => (
  <ListItem>
    <Flex justifyContent="start" className="space-x-4">
      {icon}
      <Text className="text-neutral-400">{name}</Text>
    </Flex>
    {loading ? (
      <div className="h-2.5 rounded-full bg-neutral-800 w-36 mb-2.5 animate-pulse"></div>
    ) : (
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
    )}
  </ListItem>
);

const InstanceCard = ({
  creds,
  status,
  onCreateInstance,
  isLoading,
}: InstanceCardProps) => {
  const deployStatus = status?.deploy_status;
  const isCreating = deployStatus === DeployStatus.PENDING;
  const noDatabaseCreated = creds?.status === 404;

  if (creds?.status === 500) {
    return <Error />;
  }

  if (noDatabaseCreated) {
    return (
      <Card>
        <Flex flexDirection="col" alignItems="start" className="space-y-6">
          <Title className="text-neutral-100">My Instance</Title>
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
  }

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
            loading={isLoading}
          />
          <CredentialsListItem
            name="Host"
            value={creds?.host}
            icon={<ServerIcon className="w-4 text-blue-400" />}
            hide={false}
            loading={isLoading}
          />
          <CredentialsListItem
            name="Dashboard"
            value={creds?.dbname}
            icon={<ServerStackIcon className="w-4 text-indigo-400" />}
            hide={false}
            loading={isLoading}
          />
          <CredentialsListItem
            name="User"
            value={creds?.user}
            icon={<UserIcon className="w-4 text-pink-400" />}
            hide={false}
            loading={isLoading}
          />
          <CredentialsListItem
            name="Port"
            value={creds?.port}
            icon={<ArrowRightIcon className="w-4 text-emerald-400" />}
            hide={false}
            loading={isLoading}
          />
        </List>
      </Flex>
    </Card>
  );
};

const Index = () => {
  const { data: creds } = useSWR(DATABASE_CREDENTIALS_URL, fetcher);
  const { data: status } = useSWR(DATABASE_STATUS_URL, fetcher);
  // const { data: user } = useSWR(USER_URL, fetcher);

  const deployStatus = status?.deploy_status;
  const credsRef = useRef(creds);
  const statusRef = useRef(deployStatus);
  const isLoading = !creds || !status;
  const noDatabaseCreated = creds?.status === 404;

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
    <div>
      <Flex>
        <Title className="text-gray-100">Database</Title>
        {!noDatabaseCreated && (
          <DeleteInstanceButton
            onDeleteInstance={onDeleteInstance}
            disabled={isLoading}
          />
        )}
      </Flex>
      <Grid numItemsLg={10} className="gap-8 h-full mt-6">
        <Col numColSpanLg={6} className="h-full">
          <InstanceCard
            creds={creds}
            status={status}
            onCreateInstance={onCreateInstance}
            isLoading={isLoading}
          />
        </Col>
        {!noDatabaseCreated && (
          <Col numColSpanLg={4} className="h-full space-y-8">
            <Card>
              <Flex>
                <Title className="text-neutral-100">
                  Connect with <code>psql</code>
                </Title>
                {databaseReady && <CopyToClipboardButton text={formatPsql()} />}
              </Flex>
              {databaseReady ? (
                <div className="mt-6">
                  <code className="text-emerald-400 text-sm">
                    {formatPsql()}
                  </code>
                </div>
              ) : (
                <>
                  <div className="h-2.5 rounded-full bg-neutral-800 w-36 mb-2.5 animate-pulse mt-6"></div>
                  <div className="h-2.5 rounded-full bg-neutral-800 w-full mb-2.5 animate-pulse mt-2"></div>
                </>
              )}
            </Card>
          </Col>
        )}
      </Grid>
      {!noDatabaseCreated && (
        <>
          <Flex>
            <Title className="text-gray-100 mt-12">Plan</Title>
            <ConfigureInstanceButton
              disabled={isLoading}
              onConfigureInstance={() => {}}
            />{" "}
          </Flex>
          <Text className="text-gray-300 mt-4">
            Your ParadeDB database is on the <Bold>Free Plan</Bold>.
          </Text>
          <Grid numItems={4} className="gap-x-6 gap-y-6 mt-6">
            <Col numColSpan={1}>
              <Card>
                <Icon icon={CpuChipIcon} variant="light" color="neutral" />
                <Metric className="mt-6 text-gray-100">4</Metric>
                <Text className="mt-2 text-neutral-500">CPU cores</Text>
              </Card>
            </Col>
            <Col numColSpan={1}>
              <Card>
                <Icon
                  icon={Square3Stack3DIcon}
                  variant="light"
                  color="neutral"
                />
                <Metric className="mt-6 text-gray-100">8</Metric>
                <Text className="mt-2 text-neutral-500">GB RAM</Text>
              </Card>
            </Col>
            <Col numColSpan={1}>
              <Card>
                <Icon icon={ServerIcon} variant="light" color="neutral" />
                <Metric className="mt-6 text-gray-100">256</Metric>
                <Text className="mt-2 text-neutral-500">GB Storage</Text>
              </Card>
            </Col>
          </Grid>
        </>
      )}
    </div>
  );
};

export default withPageAuthRequired(Index);
