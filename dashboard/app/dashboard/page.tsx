import { withPageAuthRequired } from "@auth0/nextjs-auth0";
import { getSession } from "@auth0/nextjs-auth0";
import {
  Card,
  Title,
  Grid,
  Col,
  Text,
  Button,
  Flex,
  List,
  ListItem,
  Bold,
} from "@tremor/react";

const DATABASE_CREDENTIALS_URL = `${process.env.PROVISIONER_URL}/database/credentials`;

const getDatabaseCredentials = async (accessToken: string | undefined) =>
  await fetch(DATABASE_CREDENTIALS_URL, {
    method: "GET",
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
  });

const Dashboard = async () => {
  const session = await getSession();
  const response = await getDatabaseCredentials(session?.accessToken);
  const creds = response.ok ? await response.json() : null;

  return (
    <Grid numItemsLg={2} className="gap-6 h-full">
      <Col numColSpanLg={2} className="h-full">
        <Card
          decoration="top"
          decorationColor="emerald"
          className="shadow-none"
        >
          <Title>My Instance</Title>
          {!creds ? (
            <List className="mt-2">
              <ListItem>
                <Bold>Host</Bold>
                <span>{creds?.host}</span>
              </ListItem>
              <ListItem>
                <Bold>User</Bold>
                <span>{creds?.user}</span>
              </ListItem>
              <ListItem>
                <Bold>Password</Bold>
                <span>{creds?.password}</span>
              </ListItem>
              <ListItem>
                <Bold>Port</Bold>
                <span>{creds?.port}</span>
              </ListItem>
            </List>
          ) : (
            <Flex flexDirection="col" className="space-y-4" alignItems="start">
              <Text className="mt-2">
                You have not created a database instance.
              </Text>
              <Button color="emerald" size="lg">
                Create Instance
              </Button>
            </Flex>
          )}
        </Card>
      </Col>
    </Grid>
  );
};

export default withPageAuthRequired(Dashboard, { returnTo: "/dashboard" });
