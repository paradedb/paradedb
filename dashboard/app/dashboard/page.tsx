import { withPageAuthRequired } from "@auth0/nextjs-auth0";
import { getSession } from "@auth0/nextjs-auth0";
import { Card, Title, Grid, Col, Text, Button, Flex } from "@tremor/react";

const DATABASE_CREDENTIALS_URL = `${process.env.PROVISIONER_URL}/database/credentials`;

const getDatabaseCredentials = async (accessToken: string | undefined) =>
  await fetch(DATABASE_CREDENTIALS_URL, {
    method: "GET",
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
  });

const Index = async () => {
  const session = await getSession();
  const user = session?.user;
  const accessToken = session?.accessToken;
  const response = await getDatabaseCredentials(accessToken);

  const creds = response.ok ? await response.json() : null;
  const hasInstance = !creds;

  return (
    <div>
      <Grid numItemsLg={2} className="gap-6">
        <Col numColSpanLg={1}>
          <Card
            decoration="top"
            decorationColor="slate"
            className="shadow-none"
          >
            <Title>Instances</Title>
            {hasInstance ? (
              <>
                <Text>Credentials here</Text>
              </>
            ) : (
              <Flex
                flexDirection="col"
                className="space-y-4"
                alignItems="start"
              >
                <Text className="mt-2">
                  You have not created any instances.
                </Text>
                <Button color="emerald" size="lg">
                  Create Instance
                </Button>
              </Flex>
            )}
          </Card>
        </Col>
        <Col numColSpanLg={1}>
          <Card
            decoration="top"
            decorationColor="slate"
            className="shadow-none"
          >
            <Title>Getting Started</Title>
          </Card>
        </Col>
      </Grid>
    </div>
  );
};

export default withPageAuthRequired(Index, { returnTo: "/dashboard" });
