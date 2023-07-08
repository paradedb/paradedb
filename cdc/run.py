from cdc.source_postgres import PostgresCDC

## TODO: Pipe these into the CDC server
dbname = "postgres"
user = "postgres"
host = "postgres-instance-1.chqsp2e4eplp.us-east-2.rds.amazonaws.com"
password = "Password123!"
table = "ecoinvent_with_types"
slot_name = "resync_slot"
port = 5432
output_plugin = "pgoutput"
publication_name = "test_pub"

try:
    print("Starting...")
    cdc = PostgresCDC(
        publication_name=publication_name,
        slot_name=slot_name,
        database=dbname,
        host=host,
        user=user,
        password=password,
        port=port,
    )

    for event in cdc:
        print(event)

except KeyboardInterrupt:
    print("\nKeyboardInterrupt caught, tearing down...")
    cdc.teardown()