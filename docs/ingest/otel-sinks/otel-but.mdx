# Exporting Open Telemetry Data to PostgreSQL(ParadeDB)

ParadeDB(PostgreSQL) has an unofficial exporter compatible with OpenTelemetry that allows you to store your otel data and use the features of 
paradedb to work with them.

Supported OTEL data: 
 - [x] Logs
 - [x] Traces
 - [ ] Metrics

## Using with OpenTelemetry Setup

To use with your opentelemetry setup you have to first add the postgresexporter in the list of exporters for your setup: 

```yaml
exporters:
  postgresexporter:
    username: "postgres"
    password: "postgres"
    database: "postgres"
    logs_table_name: "otellogs"
    traces_table_name: "oteltraces"
    metrics_table_name: "otelmetrics"
    port: 5432
    host: "localhost"
```
**Note**: Before using the exporter please ensure that the database specified in the config already exists.

You can specify the exporters to use in the service section of your config as follows:

```yaml
service:
  pipelines:
    logs:
      receivers: [otlp]
      exporters:
        - debug
        - postgresexporter
    traces:
      receivers: [otlp]
      exporters:
        - debug
        - postgresexporter
```

## Issues

In case you face any issues or have any suggestions for improving this exporter, please feel free to raise them here: 

https://github.com/destrex271/postgresexporter/issues